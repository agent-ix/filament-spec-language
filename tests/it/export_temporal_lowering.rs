// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-813 lowering of temporal clauses and fairness (FSL FR-1015,
//! FR-1016, FR-1017) over FR-756-AC-1's counter: the exact text of the
//! clause's definitions in each target, and the clause shapes the TLA+
//! encoding does not cover.

use crate::plan_export_surface::{counter_model, counter_request, source, COUNTER_UNIT};
use ix_trace_rs::trace;
use serde_json::{json, Value};

const HEAD: &str =
    "temporal T using t over (c: Counter::Counter) clock \"model-steps\" on origin {";

#[derive(Clone, Copy)]
enum Target {
    Tla,
    Lean,
}

fn checked_protocol_identity_and_locus(tail: &str) -> (String, Value) {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("counter.json"), counter_model()).unwrap();
    let unit = COUNTER_UNIT.replace(
        "model Counter",
        "profile t = \"filament.temporal.infinite-trace/v1\";\n\
         profile P = \"filament.protocol.finite-global/v1\";\n\
         model Counter",
    );
    std::fs::write(
        directory.path().join("unit.native"),
        format!("{unit}{tail}"),
    )
    .unwrap();
    let request = json!({
        "format": "native-compile/1",
        "request": {
            "models": [{"format": "semantic-ir/2.0.0", "source": {
                "file": "counter.json", "authority": "agent-ix",
                "identity": "example/counter-third", "document": "Counter",
                "formal_revision": 1
            }}],
            "program": {"source": source("unit.native")}
        }
    });
    let path = directory.path().join("compile.json");
    std::fs::write(&path, request.to_string()).unwrap();
    let bytes = fsl_command::command::compile(&path).expect("the checked fixture compiles");
    let package: Value = serde_json::from_slice(&bytes).unwrap();
    let node = package["semantic_graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["node_tag"] == "protocol" && node["semantic_form"] == "protocol_clause")
        .expect("the checked graph contains Hand");
    let node_id = node["node_id"].clone();
    let checked_node_id: filament_contract_model::CheckedNodeId =
        serde_json::from_value(node_id.clone()).unwrap();
    let identity = fsl_replay::protocol_analysis::node_text(&checked_node_id).unwrap();
    let mut locus = package["source_map"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["node_id"] == node_id)
        .and_then(|entry| entry["regions"].as_array())
        .and_then(|regions| regions.first())
        .map(|region| region.clone())
        .unwrap_or(Value::Null);
    if locus.is_object() {
        locus["kind"] = json!("region");
    }
    (identity, locus)
}

impl Target {
    fn profile(self) -> &'static str {
        match self {
            Self::Tla => "filament.output.tla2/v1",
            Self::Lean => "filament.output.lean4/v1",
        }
    }
}

/// Exports the clause `body` of the counter to `target`: the exit code and
/// the result document.
fn export(body: &str, target: Target) -> (u8, Value) {
    export_from(body, target, None)
}

/// As [`export`], from an initial snapshot whose counter holds `start`.
fn export_from(body: &str, target: Target, start: Option<i64>) -> (u8, Value) {
    export_full(
        body,
        target,
        start,
        "filament.temporal.infinite-trace/v1",
        true,
    )
}

/// As [`export`], with no `initial` snapshot: the request fixes no keys.
fn export_unfixed(body: &str, target: Target) -> (u8, Value) {
    export_full(
        body,
        target,
        None,
        "filament.temporal.infinite-trace/v1",
        false,
    )
}

/// As [`export_from`], under the temporal profile `profile`.
fn export_full(
    body: &str,
    target: Target,
    start: Option<i64>,
    profile: &str,
    fixed: bool,
) -> (u8, Value) {
    export_unit(
        &format!("{HEAD} {body} }}\n"),
        target,
        start,
        profile,
        fixed,
    )
}

/// Exports the declarations `tail`, which follow the counter's, to `target`
/// from the counter's snapshot.
fn export_tail(tail: &str, target: Target) -> (u8, Value) {
    export_unit_with_protocol_binding(
        tail,
        target,
        None,
        "filament.temporal.infinite-trace/v1",
        true,
        true,
        true,
    )
}

fn export_tail_with_unbound_static_role(tail: &str, target: Target) -> (u8, Value) {
    export_unit_with_protocol_binding(
        tail,
        target,
        None,
        "filament.temporal.infinite-trace/v1",
        true,
        true,
        false,
    )
}

/// Exports the obligation `T` of the unit that is the counter's followed by
/// `tail`.
fn export_unit(
    tail: &str,
    target: Target,
    start: Option<i64>,
    profile: &str,
    fixed: bool,
) -> (u8, Value) {
    export_unit_with_protocol_binding(
        tail,
        target,
        start,
        profile,
        fixed,
        tail.contains("protocol Hand"),
        true,
    )
}

fn export_unit_with_protocol_binding(
    tail: &str,
    target: Target,
    start: Option<i64>,
    profile: &str,
    fixed: bool,
    bind_protocol: bool,
    bind_roles: bool,
) -> (u8, Value) {
    export_unit_with_protocol_options(
        tail,
        target,
        start,
        profile,
        fixed,
        bind_protocol,
        bind_roles,
        "example/counter-third",
        "c",
        json!({}),
        None,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
fn export_unit_with_protocol_options(
    tail: &str,
    target: Target,
    start: Option<i64>,
    profile: &str,
    fixed: bool,
    bind_protocol: bool,
    bind_roles: bool,
    role_selection: &str,
    over_key: &str,
    capacities: Value,
    fidelity: Option<&str>,
    output: Option<&str>,
) -> (u8, Value) {
    export_unit_with_protocol_role_type(
        tail,
        target,
        start,
        profile,
        fixed,
        bind_protocol,
        bind_roles,
        role_selection,
        over_key,
        capacities,
        fidelity,
        output,
        "model-binding",
    )
}

#[allow(clippy::too_many_arguments)]
fn export_unit_with_protocol_role_type(
    tail: &str,
    target: Target,
    start: Option<i64>,
    profile: &str,
    fixed: bool,
    bind_protocol: bool,
    bind_roles: bool,
    role_selection: &str,
    over_key: &str,
    capacities: Value,
    fidelity: Option<&str>,
    output: Option<&str>,
    role_type: &str,
) -> (u8, Value) {
    export_unit_with_protocol_role_and_over_type(
        tail,
        target,
        start,
        profile,
        fixed,
        bind_protocol,
        bind_roles,
        role_selection,
        over_key,
        capacities,
        fidelity,
        output,
        role_type,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
fn export_unit_with_protocol_role_and_over_type(
    tail: &str,
    target: Target,
    start: Option<i64>,
    profile: &str,
    fixed: bool,
    bind_protocol: bool,
    bind_roles: bool,
    role_selection: &str,
    over_key: &str,
    capacities: Value,
    fidelity: Option<&str>,
    output: Option<&str>,
    role_type: &str,
    over_object_type: Option<&str>,
) -> (u8, Value) {
    let directory = tempfile::tempdir().unwrap();
    let mut export = json!({"profile": target.profile(), "obligations": ["T"]});
    if let Some(fidelity) = fidelity {
        export["minimum_fidelity"] = json!(fidelity);
    }
    if let Some(output) = output {
        export["output"] = json!({"file": output});
    }
    if bind_protocol && tail.contains("protocol Hand") {
        let mut roles = serde_json::Map::new();
        if bind_roles {
            for role in ["a", "b"] {
                if tail.contains(&format!("role {role} on")) {
                    roles.insert(
                        role.to_owned(),
                        json!({"type": role_type, "selection": role_selection}),
                    );
                }
            }
        }
        export["protocols"] = json!([{
            "protocol": "Hand",
            "over": {"reference": {
                "population": "ix://example/counter-third/counters", "key": over_key
            }},
            "roles": roles,
            "capacities": capacities,
        }]);
    }
    let path = counter_request(directory.path(), export);
    if let Some(object_type) = over_object_type {
        let file = directory.path().join("counter-0.json");
        let mut snapshot: Value = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
        let object = snapshot["populations"][0]["objects"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|object| object["key"] == over_key)
            .expect("the over object exists in the fixture");
        object["type"] = json!(object_type);
        std::fs::write(&file, serde_json::to_vec(&snapshot).unwrap()).unwrap();
    }
    if let Some(start) = start {
        let file = directory.path().join("counter-0.json");
        let snapshot = std::fs::read_to_string(&file).unwrap();
        let zero = "\"integer\":\"0\"";
        assert!(snapshot.contains(zero), "{snapshot}");
        std::fs::write(
            &file,
            snapshot.replace(zero, &format!("\"integer\":\"{start}\"")),
        )
        .unwrap();
    }
    if !fixed {
        let file = path.clone();
        let mut request: Value = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
        request["request"]["export"]
            .as_object_mut()
            .unwrap()
            .remove("initial");
        std::fs::write(&file, serde_json::to_vec(&request).unwrap()).unwrap();
    }
    let unit = COUNTER_UNIT.replace(
        "model Counter",
        &format!(
            "profile t = \"{profile}\";\nprofile P = \"filament.protocol.finite-global/v1\";\nmodel Counter"
        ),
    );
    std::fs::write(
        directory.path().join("unit.native"),
        format!("{unit}{tail}"),
    )
    .unwrap();
    let result = match fsl_command::command::run(&path, &crate::negotiation::Fr290) {
        Ok(result) => (
            result.exit_code,
            serde_json::from_str(&result.value.to_string()).unwrap(),
        ),
        Err(error) => (error.category().exit_code(), error.value().unwrap()),
    };
    if let Some(output) = output {
        assert!(
            !directory.path().join(output).exists(),
            "refused export wrote its requested file"
        );
    }
    result
}

fn export_two_model_protocol(
    role_declarations: &str,
    roles: Value,
    over_type: &str,
) -> (u8, Value) {
    use crate::support::refinement_cas::{self, Variant};

    let directory = tempfile::tempdir().unwrap();
    let mut export = json!({"profile": Target::Tla.profile(), "obligations": ["T"],
        "protocols": [{"protocol": "Hand", "over": {"reference": {
            "population": "ix://example/counter-third/counters", "key": "c"
        }}, "roles": roles, "capacities": {}}]});
    let request_path = counter_request(directory.path(), export.take());
    let documents = refinement_cas::documents(Variant::CAS);
    std::fs::write(directory.path().join("spec.json"), &documents[0]).unwrap();
    let tail = format!(
        "{}protocol Hand using P over (k: {over_type}) on origin {{\n\
             {role_declarations}\
             requires temporal T;\n\
             run sequence Main {{ check Ok using v {{ true }}; }}\n\
             finish Done as (d: Counter::Counter) {{ true }};\n\
         }}\n",
        clause_tail("always holds(c.value <= 5)")
    );
    let unit = COUNTER_UNIT.replace(
        "model Counter",
        "profile t = \"filament.temporal.infinite-trace/v1\";\n\
         profile P = \"filament.protocol.finite-global/v1\";\n\
         model Counter",
    );
    let unit = unit.replace(
        "model Counter = \"example/counter-third\";\n",
        "model Counter = \"example/counter-third\";\n\
         model Spec = \"example/counter-spec\";\n",
    );
    let unit = format!("{unit}{tail}");
    std::fs::write(directory.path().join("unit.native"), unit).unwrap();
    let mut request: Value =
        serde_json::from_slice(&std::fs::read(&request_path).unwrap()).unwrap();
    request["request"]["models"].as_array_mut().unwrap().push(
        json!({"format": "semantic-ir/2.0.0", "source": {
            "file": "spec.json", "authority": "agent-ix",
            "identity": "example/counter-spec", "document": "Counter",
            "formal_revision": 1
        }}),
    );
    std::fs::write(&request_path, serde_json::to_vec(&request).unwrap()).unwrap();
    match fsl_command::command::run(&request_path, &crate::negotiation::Fr290) {
        Ok(result) => (
            result.exit_code,
            serde_json::from_str(&result.value.to_string()).unwrap(),
        ),
        Err(error) => (error.category().exit_code(), error.value().unwrap()),
    }
}

fn exported_text(body: &str, target: Target) -> String {
    exported_text_from(body, target, None)
}

mod dense_time {
    use super::{counter_request, json, Target, Value};
    use crate::plan_export_surface::counter_model;
    use ix_trace_rs::trace;
    use std::path::{Path, PathBuf};

    fn lamp(body: &str, target: Target, on: bool) -> (u8, Value) {
        let (_dir, path) = lamp_request(body, target, on);
        run_lamp(&path)
    }

    fn lamp_request(body: &str, target: Target, on: bool) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = counter_request(
            dir.path(),
            json!({"profile": target.profile(), "obligations": ["T"]}),
        );
        let model = String::from_utf8(counter_model())
            .unwrap()
            .replace("Counter", "Lamp");
        let mut model: Value = serde_json::from_str(&model).unwrap();
        let object = &mut model["types"][1];
        let prefix = "ix://example/counter-third/Lamp";
        let mut on_field = object["fields"][0].clone();
        on_field["identity"] = json!(format!("{prefix}/on"));
        on_field["name"] = json!("on");
        on_field["typeRef"] = json!("ix://filament/native/Boolean");
        let mut clock = on_field.clone();
        clock["identity"] = json!(format!("{prefix}/x"));
        clock["name"] = json!("x");
        clock["typeRef"] = json!("ix://filament/native/Clock");
        object["fields"] = json!([on_field, clock]);
        let operation = &mut object["operations"][0];
        operation["identity"] = json!(format!("{prefix}/release"));
        operation["name"] = json!("release");
        operation["frame"]["modifies"] = json!([format!("{prefix}/on"), format!("{prefix}/x")]);
        std::fs::write(dir.path().join("counter.json"), model.to_string()).unwrap();
        let snapshot_path = dir.path().join("counter-0.json");
        let mut snapshot: Value =
            serde_json::from_slice(&std::fs::read(&snapshot_path).unwrap()).unwrap();
        let row = &mut snapshot["populations"][0]["objects"][0];
        row["type"] = json!(prefix);
        row["fields"] = json!({"on": {"boolean": on}});
        std::fs::write(snapshot_path, snapshot.to_string()).unwrap();
        let unit = format!(
            "language \"ix:native\" edition \"1-draft\";\n\
             profile v = \"filament.value.complete/v1\";\n\
             profile c = \"filament.temporal.complete/v1\";\n\
             profile t = \"filament.temporal.timed/v1\";\n\
             model Lamp = \"example/counter-third\";\n\
             dimension Time; unit ms : Time = rational(1, 1); time Lamp dense unit ms;\n\
             pre CanRelease using v on Lamp::Lamp::release {{ self.on and self.x >= 2 ms }}\n\
             post Release using v on Lamp::Lamp::release {{ not self.on and self.x = 0 ms }}\n\
             time invariant WhileOn when {{ self.on }} {{ self.x <= 5 ms }}\n\
             temporal T using t over (l: Lamp::Lamp) clock \"model-time\" on origin {{ {body} }}\n"
        );
        std::fs::write(dir.path().join("unit.native"), unit).unwrap();
        (dir, path)
    }

    fn run_lamp(path: &Path) -> (u8, Value) {
        match fsl_command::command::run(path, &crate::negotiation::Fr290) {
            Ok(result) => (
                result.exit_code,
                serde_json::from_str(&result.value.to_string()).unwrap(),
            ),
            Err(error) => (error.category().exit_code(), error.value().unwrap()),
        }
    }

    fn lamp_checked_node(path: &Path, find: impl Fn(&Value) -> bool) -> Value {
        let directory = path.parent().unwrap();
        let mut compile: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        compile["format"] = json!("native-compile/1");
        let request = &mut compile["request"];
        request.as_object_mut().unwrap().remove("export");
        request["output"] = json!({"package": "package.json"});
        request["handoff"] = json!({
            "directory": "handoff", "package_name": "test:w21:unit.native",
            "native_join": {"work_units": 1_u64 << 40, "storage": 1_u64 << 24}
        });
        let compile_path = directory.join("lamp-compile.json");
        std::fs::write(&compile_path, compile.to_string()).unwrap();
        fsl_command::command::compile(&compile_path).expect("the Lamp checked package compiles");
        let checked: Value =
            serde_json::from_slice(&std::fs::read(directory.join("package.json")).unwrap())
                .unwrap();
        let (row, node) = checked["semantic_graph"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .find(|(_, node)| find(node))
            .expect("the checked graph has the expected node");
        // FR-322 gives export outcomes document-held IDs. Resolve this node's
        // operand slots through its own row before comparing the outcome.
        let graph = fsl_semantics::check::wire_id::WireGraph::of_package_value(&checked)
            .expect("the checked package has a valid operand table");
        graph
            .document_value_in_row(row, node)
            .expect("the checked node's operand slots resolve to graph indexes")
    }

    fn node_text(checked: &Value) -> String {
        let id: fsl_package::CheckedNodeId =
            serde_json::from_value(checked["node_id"].clone()).unwrap();
        fsl_replay::protocol_analysis::node_text(&id).unwrap()
    }

    #[trace("FSpec-FR-813-AC-18", "FSpec-TC-1972")]
    #[test]
    fn a_dense_subject_emits_clock_updates_urgency_and_quiescence() {
        let (_dir, path) = lamp_request("always holds(l.on or not l.on)", Target::Tla, true);
        let (exit, tla) = run_lamp(&path);
        assert_eq!(exit, 0, "{tla}");
        let text = tla["export"]["target_text"].as_str().unwrap();
        for expected in [
            "TLC, Reals",
            "VARIABLES v_Lamp, now, tick, idle, k_Lamp_x",
            "Tick(d) ==",
            "TimeInv(d)",
            "k_Lamp_x[k] + d <= 5",
            "Quiescent == ~Urgent /\\ ~(\\E k \\in DOMAIN v_Lamp : v_Lamp[k].fld_on)",
            "k_Lamp_x[r_self] >= 2",
            "k_Lamp_x' = [k_Lamp_x EXCEPT ![r_self] = 0]",
            "now' = now",
            "tick' = FALSE",
            "idle' = TRUE",
            "NZ == \\A r \\in Real : <>(now > r)",
        ] {
            assert!(text.contains(expected), "missing {expected}: {text}");
        }
        let record = &tla["export"]["records"][0];
        assert_eq!(record["disposition"], "conditional", "{record}");
        let [condition] = record["conditions"].as_array().unwrap().as_slice() else {
            panic!("one condition: {record}");
        };
        assert_eq!(condition["type"], "real-valued-time", "{condition}");
        assert_eq!(condition["clause"], "T", "{condition}");
        let root = lamp_checked_node(&path, |node| {
            node["node_tag"] == "timed_model" && node["semantic_form"] == "timed_model"
        });
        assert_eq!(condition["node"], node_text(&root), "{condition}");
        assert_clock_name_map(&tla, Target::Tla);
        let (exit, lean) = lamp("always holds(l.on or not l.on)", Target::Lean, true);
        assert_eq!(exit, 0, "{lean}");
        let text = lean["export"]["target_text"].as_str().unwrap();
        for expected in [
            "ts : Nat -> Q",
            "ck : Nat -> Clocks",
            "F_clockWf",
            "F_clockInit",
            "F_clockAdvance",
            "TimeInv",
            "nz : forall r : Q",
        ] {
            assert!(text.contains(expected), "missing {expected}: {text}");
        }
        assert_eq!(
            lean["export"]["records"][0]["disposition"], "preserved",
            "{lean}"
        );
        assert_eq!(
            lean["export"]["records"][0]["conditions"],
            json!([]),
            "{lean}"
        );
        assert_clock_name_map(&lean, Target::Lean);
    }

    fn assert_clock_name_map(value: &Value, target: Target) {
        let names = value["export"]["name_map"].as_array().unwrap();
        assert!(names
            .iter()
            .any(|entry| entry["name"] == "k_Lamp_x" && entry["node"].is_string()));
        let fixed = "now tick idle TimeInv Tick Idle Urgent Quiescent TNext NZ Q Clocks TBehaviour";
        for fixed in fixed.split_ascii_whitespace() {
            assert!(
                !names.iter().any(|entry| entry["name"] == fixed),
                "{fixed}: {names:?}"
            );
        }
        if matches!(target, Target::Lean) {
            let helpers =
                "Q_normalize Q_add Q_sub Q_le Q_lt Q_eq Q_wf F_clockWf F_clockInit F_clockAdvance";
            for helper in helpers.split_ascii_whitespace() {
                let mut entries = names.iter().filter(|entry| entry["name"] == helper);
                let entry = entries
                    .next()
                    .unwrap_or_else(|| panic!("missing helper {helper}"));
                assert!(
                    entries.next().is_none()
                        && entry.get("binding").is_none()
                        && entry["node"].is_string(),
                    "{entry}"
                );
            }
        }
    }

    fn assert_operator_refusal(body: &str, target: Target, operation: &str) {
        let (_dir, path) = lamp_request(body, target, true);
        let (exit, value) = run_lamp(&path);
        assert_eq!(exit, 21, "{value}");
        let refusal = &value["export"]["refusal"];
        assert_eq!(refusal["code"], "unimplemented_capability", "{refusal}");
        assert_eq!(refusal["cause"], "unimplemented-meaning", "{refusal}");
        let checked = lamp_checked_node(&path, |node| {
            node["body"]["operation"]["identity"] == operation
        });
        let construct = format!(
            "{}/{}",
            checked["node_tag"].as_str().unwrap(),
            checked["semantic_form"].as_str().unwrap()
        );
        assert_eq!(refusal["node"], node_text(&checked), "{refusal}");
        assert_eq!(refusal["construct"], construct, "{refusal}");
        let family = match target {
            Target::Tla => "tla",
            Target::Lean => "lean",
        };
        assert_eq!(
            refusal["capability"],
            format!("export:{family}:{construct}"),
            "{refusal}"
        );
        assert!(value["export"]["target_text"].is_null(), "{value}");
    }

    #[trace("FSpec-FR-813-AC-24", "FSpec-TC-1972")]
    #[test]
    fn dense_tla_refuses_the_root_until_at_its_checked_operator_node() {
        assert_operator_refusal(
            "holds(l.on) until holds(not l.on)",
            Target::Tla,
            "filament.op.temporal.until",
        );
    }

    #[trace("FSpec-FR-813-AC-27", "FSpec-TC-1972")]
    #[test]
    fn dense_liveness_reads_positions_and_requires_time_divergence() {
        let (exit, tla) = lamp("always eventually holds(not l.on)", Target::Tla, false);
        assert_eq!(exit, 0, "{tla}");
        let text = tla["export"]["target_text"].as_str().unwrap();
        assert!(text.contains("NZ /\\ []<>~tick"), "{text}");
        assert!(text.contains("[](tick \\/ <>(~tick /\\ "), "{text}");
    }

    #[trace("FSpec-FR-813-AC-19", "FSpec-FR-813-AC-27", "FSpec-TC-1972")]
    #[test]
    fn an_origin_window_reads_positions_after_idle_and_never_after_tick() {
        for (body, premise, lo, hi) in [
            (
                "eventually[0 ms, 5 ms] holds(not l.on)",
                "[]<>accept_T(o_l)",
                0,
                5,
            ),
            ("always[2 ms, 3 ms] holds(l.on)", "[]accept_T(o_l)", 2, 3),
        ] {
            let (exit, tla) = lamp(body, Target::Tla, lo == 0);
            assert_eq!(exit, 0, "{tla}");
            let text = tla["export"]["target_text"].as_str().unwrap();
            assert!(
                text.contains(&format!("({lo} <= now' /\\ now' <= {hi})")),
                "{text}"
            );
            assert!(text.contains("Next_T == ((Next \\/ \\E d \\in Real : Idle(d)) /\\ Mon_T) \\/ (\\E d \\in Real : Tick(d) /\\ UNCHANGED <<m_T>>)"), "{text}");
            let theorem = format!(
                r"THEOREM Th_T == Spec_T => ((NZ /\ []<>~tick) => \A o_l \in Ids_Lamp : [](ok_T(o_l)) /\ {premise})"
            );
            assert!(text.lines().any(|line| line == theorem), "{text}");
            let (exit, lean) = lamp(body, Target::Lean, lo == 0);
            assert_eq!(exit, 0, "{lean}");
            let text = lean["export"]["target_text"].as_str().unwrap();
            assert!(
                text.contains(&format!("Q_le (Q.mk {lo} 1) (Q_sub (b.ts")),
                "{text}"
            );
            assert!(text.contains(&format!("(Q.mk {hi} 1)")), "{text}");
        }
    }

    #[trace("FSpec-FR-813", "FSpec-TC-1972")]
    #[test]
    fn a_window_under_always_tracks_each_open_start() {
        for (body, reject) in [
            (
                "always eventually[0 ms, 5 ms] holds(not l.on)",
                "now' - s > 5",
            ),
            (
                "always always[2 ms, 3 ms] holds(l.on)",
                "(2 <= now' - s /\\ now' - s <= 3)",
            ),
        ] {
            let (exit, tla) = lamp(body, Target::Tla, true);
            assert_eq!(exit, 0, "{tla}");
            let text = tla["export"]["target_text"].as_str().unwrap();
            for expected in [
                "m_T_s0 \\in [Ids_Lamp -> SUBSET Real]",
                "m_T_s0[o_l] \\cup {now'}",
                "m_T_r0[o_l] \\/",
                reject,
                "Tick(d) /\\ UNCHANGED <<m_T_s0, m_T_r0>>",
                "[]accept_T(o_l)",
            ] {
                assert!(text.contains(expected), "missing {expected}: {text}");
            }
            let (exit, lean) = lamp(body, Target::Lean, true);
            assert_eq!(exit, 0, "{lean}");
            let text = lean["export"]["target_text"].as_str().unwrap();
            assert!(text.contains("forall d1 : Int"), "{text}");
            assert!(
                text.contains(
                    "Q_sub (b.ts (Int.toNat ((i + d1) + d2))) (b.ts (Int.toNat (i + d1)))"
                ),
                "{text}"
            );
        }
    }

    #[trace("FSpec-FR-813-AC-24", "FSpec-TC-1972")]
    #[test]
    fn a_timed_window_under_eventually_keeps_its_typed_refusal() {
        let body = "eventually always[0 ms, 5 ms] holds(l.on)";
        for target in [Target::Tla, Target::Lean] {
            assert_operator_refusal(body, target, "filament.op.temporal.always");
        }
    }

    #[trace("FSpec-FR-813-AC-24", "FSpec-FR-814", "FSpec-TC-1972")]
    #[test]
    fn dense_tla_refuses_the_second_timed_window_at_its_checked_node() {
        for body in [
            "always[0 ms, 5 ms] holds(l.on) and eventually[1 ms, 4 ms] holds(not l.on)",
            "always (always[0 ms, 5 ms] holds(l.on) and eventually[1 ms, 4 ms] holds(not l.on))",
        ] {
            assert_operator_refusal(body, Target::Tla, "filament.op.temporal.eventually");
            let (exit, lean) = lamp(body, Target::Lean, true);
            assert_eq!(exit, 0, "{lean}");
            assert_eq!(lean["export"]["records"][0]["disposition"], "preserved");
        }
    }
}

fn exported_text_from(body: &str, target: Target, start: Option<i64>) -> String {
    let (exit, value) = export_from(body, target, start);
    assert_eq!(exit, 0, "{value}");
    value["export"]["target_text"]
        .as_str()
        .expect("target text")
        .to_owned()
}

/// The definitions the clause adds, without their `fsl:` comments: from the
/// first of the clause's own definitions to the end of the text.
fn clause_text(body: &str, target: Target) -> String {
    clause_text_from(body, target, None)
}

fn clause_text_from(body: &str, target: Target, start: Option<i64>) -> String {
    clause_of(&exported_text_from(body, target, start), target)
}

/// As [`clause_text`], for a request that fixes no keys.
fn clause_text_unfixed(body: &str, target: Target) -> String {
    let (exit, value) = export_unfixed(body, target);
    assert_eq!(exit, 0, "{value}");
    clause_of(value["export"]["target_text"].as_str().unwrap(), target)
}

/// The clause's definitions of `text`, from the identity universe on.
fn clause_of(text: &str, target: Target) -> String {
    let starts: &[&str] = match target {
        Target::Tla => &["Ids_Counter ==", "ok_T"],
        Target::Lean => &["def Ids_Counter"],
    };
    let from = starts
        .iter()
        .filter_map(|start| text.find(start))
        .min()
        .expect("the clause's definitions");
    text[from..]
        .lines()
        .filter(|line| !line.starts_with("\\* fsl: ") && !line.starts_with("-- fsl: "))
        .collect::<Vec<_>>()
        .join("\n")
}

/// FSpec FR-813-AC-1: always as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-1")]
fn always_tla() {
    assert_eq!(
        clause_text("always holds(c.value <= 5)", Target::Tla),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter

THEOREM Th_T == Spec => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ []((v_Counter[o_c].fld_value <= 5))

===="#
    );
}

/// FSpec FR-813-AC-1: always as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-1")]
fn always_lean() {
    assert_eq!(
        clause_text("always holds(c.value <= 5)", Target::Lean),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value <= 5)

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 0 <= d1 -> (0 <= (i + d1) /\ at_T_0 (b.st (Int.toNat (i + d1))) o_c))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-7: `always` from a request that fixes no keys as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-7")]
fn always_unfixed_keys_tla() {
    assert_eq!(
        clause_text_unfixed("always holds(c.value <= 5)", Target::Tla,),
        r#"Ids_Counter == Keys_Counter

ok_T(o_c) == o_c \in DOMAIN v_Counter

THEOREM Th_T == Spec => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ []((v_Counter[o_c].fld_value <= 5))

===="#
    );
}

/// FSpec FR-813-AC-7: `always` from a request that fixes no keys as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-7")]
fn always_unfixed_keys_lean() {
    assert_eq!(
        clause_text_unfixed("always holds(c.value <= 5)", Target::Lean,),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value <= 5)

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 0 <= d1 -> (0 <= (i + d1) /\ at_T_0 (b.st (Int.toNat (i + d1))) o_c))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-7: until as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-7")]
fn until_tla() {
    assert_eq!(
        clause_text("holds(c.value >= 1) until holds(c.value >= 3)", Target::Tla),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter

vars_T == <<vars, m_T>>

TypeOK_T ==
  /\ m_T \in [Ids_Counter -> {"run", "acc", "rej"}]

Init_T ==
  /\ Init
  /\ m_T = [o_c \in Ids_Counter |-> IF (v_Counter[o_c].fld_value >= 3) THEN "acc" ELSE IF ~(v_Counter[o_c].fld_value >= 1) THEN "rej" ELSE "run"]
  /\ TypeOK_T

Mon_T ==
  /\ m_T' = [o_c \in Ids_Counter |-> IF m_T[o_c] /= "run" THEN m_T[o_c] ELSE IF (v_Counter'[o_c].fld_value >= 3) THEN "acc" ELSE IF ~(v_Counter'[o_c].fld_value >= 1) THEN "rej" ELSE "run"]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

accept_T(o_c) == m_T[o_c] = "acc"

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ []<>accept_T(o_c)

===="#
    );
}

/// FSpec FR-813-AC-7: until as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-7")]
fn until_lean() {
    assert_eq!(
        clause_text(
            "holds(c.value >= 1) until holds(c.value >= 3)",
            Target::Lean
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value >= 1)

def at_T_1 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value >= 3)

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (exists d1 : Int, 0 <= d1 /\ (0 <= (i + d1) /\ at_T_1 (b.st (Int.toNat (i + d1))) o_c) /\ (forall e2 : Int, 0 <= e2 -> e2 < d1 -> (0 <= (i + e2) /\ at_T_0 (b.st (Int.toNat (i + e2))) o_c)))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-7: `until` from a request that fixes no keys as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-7")]
fn until_unfixed_keys_tla() {
    assert_eq!(
        clause_text_unfixed("holds(c.value >= 1) until holds(c.value >= 3)", Target::Tla,),
        r#"Ids_Counter == Keys_Counter

ok_T(o_c) == o_c \in DOMAIN v_Counter

vars_T == <<vars, m_T>>

TypeOK_T ==
  /\ m_T \in [Ids_Counter -> {"run", "acc", "rej"}]

Init_T ==
  /\ Init
  /\ m_T = [o_c \in Ids_Counter |-> IF (v_Counter[o_c].fld_value >= 3) THEN "acc" ELSE IF ~(v_Counter[o_c].fld_value >= 1) THEN "rej" ELSE "run"]
  /\ TypeOK_T

Mon_T ==
  /\ m_T' = [o_c \in Ids_Counter |-> IF m_T[o_c] /= "run" THEN m_T[o_c] ELSE IF (v_Counter'[o_c].fld_value >= 3) THEN "acc" ELSE IF ~(v_Counter'[o_c].fld_value >= 1) THEN "rej" ELSE "run"]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

accept_T(o_c) == m_T[o_c] = "acc"

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ []<>accept_T(o_c)

===="#
    );
}

/// FSpec FR-813-AC-7: `until` from a request that fixes no keys as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-7")]
fn until_unfixed_keys_lean() {
    assert_eq!(
        clause_text_unfixed(
            "holds(c.value >= 1) until holds(c.value >= 3)",
            Target::Lean,
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value >= 1)

def at_T_1 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value >= 3)

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (exists d1 : Int, 0 <= d1 /\ (0 <= (i + d1) /\ at_T_1 (b.st (Int.toNat (i + d1))) o_c) /\ (forall e2 : Int, 0 <= e2 -> e2 < d1 -> (0 <= (i + e2) /\ at_T_0 (b.st (Int.toNat (i + e2))) o_c)))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-3: an `always` over steps 0 to 3 as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-3")]
fn interval_always_tla() {
    assert_eq!(
        clause_text("always[0, 3] holds(c.value <= 5)", Target::Tla),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter

vars_T == <<vars, n_T_0, m_T>>

TypeOK_T ==
  /\ n_T_0 \in 0..3
  /\ m_T \in [Ids_Counter -> {"run", "rel", "rej"}]

Init_T ==
  /\ Init
  /\ n_T_0 = 0
  /\ m_T = [o_c \in Ids_Counter |-> IF (0 <= 0 /\ 0 <= 3) /\ ~(v_Counter[o_c].fld_value <= 5) THEN "rej" ELSE IF 0 >= 3 THEN "rel" ELSE "run"]
  /\ TypeOK_T

Mon_T ==
  /\ n_T_0' = IF n_T_0 < 3 THEN n_T_0 + 1 ELSE 3
  /\ m_T' = [o_c \in Ids_Counter |-> IF m_T[o_c] /= "run" THEN m_T[o_c] ELSE IF (0 <= n_T_0' /\ n_T_0' <= 3) /\ ~(v_Counter'[o_c].fld_value <= 5) THEN "rej" ELSE IF n_T_0' >= 3 THEN "rel" ELSE "run"]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

accept_T(o_c) == m_T[o_c] /= "rej"

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ []accept_T(o_c)

===="#
    );
}

/// FSpec FR-813-AC-3: an `always` over steps 0 to 3 as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-3")]
fn interval_always_lean() {
    assert_eq!(
        clause_text("always[0, 3] holds(c.value <= 5)", Target::Lean),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value <= 5)

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 0 <= d1 -> d1 <= 3 -> (0 <= (i + d1) /\ at_T_0 (b.st (Int.toNat (i + d1))) o_c))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-21: two interval operators, each with its own counter as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-21")]
fn two_intervals_tla() {
    assert_eq!(
        clause_text(
            "always[0, 3] holds(c.value >= 4) and eventually[1, 2] holds(c.value = 5)",
            Target::Tla
        ),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter /\ ((0 <= 5) /\ (5 <= 9))

vars_T == <<vars, n_T_0, m_T_0, n_T_1, m_T_1>>

TypeOK_T ==
  /\ n_T_0 \in 0..3
  /\ m_T_0 \in [Ids_Counter -> {"run", "rel", "rej"}]
  /\ n_T_1 \in 0..2
  /\ m_T_1 \in [Ids_Counter -> {"run", "acc", "rej"}]

Init_T ==
  /\ Init
  /\ n_T_0 = 0
  /\ m_T_0 = [o_c \in Ids_Counter |-> IF (0 <= 0 /\ 0 <= 3) /\ ~(v_Counter[o_c].fld_value >= 4) THEN "rej" ELSE IF 0 >= 3 THEN "rel" ELSE "run"]
  /\ n_T_1 = 0
  /\ m_T_1 = [o_c \in Ids_Counter |-> IF (1 <= 0 /\ 0 <= 2) /\ (v_Counter[o_c].fld_value = 5) THEN "acc" ELSE IF 0 >= 2 THEN "rej" ELSE "run"]
  /\ TypeOK_T

Mon_T ==
  /\ n_T_0' = IF n_T_0 < 3 THEN n_T_0 + 1 ELSE 3
  /\ m_T_0' = [o_c \in Ids_Counter |-> IF m_T_0[o_c] /= "run" THEN m_T_0[o_c] ELSE IF (0 <= n_T_0' /\ n_T_0' <= 3) /\ ~(v_Counter'[o_c].fld_value >= 4) THEN "rej" ELSE IF n_T_0' >= 3 THEN "rel" ELSE "run"]
  /\ n_T_1' = IF n_T_1 < 2 THEN n_T_1 + 1 ELSE 2
  /\ m_T_1' = [o_c \in Ids_Counter |-> IF m_T_1[o_c] /= "run" THEN m_T_1[o_c] ELSE IF (1 <= n_T_1' /\ n_T_1' <= 2) /\ (v_Counter'[o_c].fld_value = 5) THEN "acc" ELSE IF n_T_1' >= 2 THEN "rej" ELSE "run"]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

accept_T_0(o_c) == m_T_0[o_c] /= "rej"

accept_T_1(o_c) == m_T_1[o_c] = "acc"

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ []accept_T_0(o_c) /\ []<>accept_T_1(o_c)

===="#
    );
}

/// FSpec FR-813-AC-21: two interval operators, each with its own counter as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-21")]
fn two_intervals_lean() {
    assert_eq!(
        clause_text(
            "always[0, 3] holds(c.value >= 4) and eventually[1, 2] holds(c.value = 5)",
            Target::Lean
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter /\ ((0 <= 5) /\ (5 <= 9))

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value >= 4)

def at_T_1 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value = 5)

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  ((forall d1 : Int, 0 <= d1 -> d1 <= 3 -> (0 <= (i + d1) /\ at_T_0 (b.st (Int.toNat (i + d1))) o_c)) /\ (exists d2 : Int, 1 <= d2 /\ d2 <= 2 /\ (0 <= (i + d2) /\ at_T_1 (b.st (Int.toNat (i + d2))) o_c)))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-6: `eventually always`, stated directly as `<>[]` as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-6")]
fn co_buchi_tla() {
    assert_eq!(
        clause_text("eventually always holds(c.value <= 5)", Target::Tla),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter

THEOREM Th_T == Spec => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ <>([]((v_Counter[o_c].fld_value <= 5)))

===="#
    );
}

/// FSpec FR-813-AC-6: `eventually always`, stated directly as `<>[]` as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-6")]
fn co_buchi_lean() {
    assert_eq!(
        clause_text("eventually always holds(c.value <= 5)", Target::Lean),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value <= 5)

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (exists d1 : Int, 0 <= d1 /\ (forall d2 : Int, 0 <= d2 -> (0 <= ((i + d1) + d2) /\ at_T_0 (b.st (Int.toNat ((i + d1) + d2))) o_c)))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-6: `eventually always` over a `since` as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-6")]
fn co_buchi_since_tla() {
    assert_eq!(
        clause_text(
            "eventually always (holds(c.value <= 5) since holds(c.value = 0))",
            Target::Tla
        ),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter /\ ((0 <= 0) /\ (0 <= 9))

vars_T == <<vars, m_T_h0>>

TypeOK_T ==
  /\ m_T_h0 \in [Ids_Counter -> BOOLEAN]

Init_T ==
  /\ Init
  /\ m_T_h0 = [o_c \in Ids_Counter |-> (v_Counter[o_c].fld_value = 0)]
  /\ TypeOK_T

Mon_T ==
  /\ m_T_h0' = [o_c \in Ids_Counter |-> ((v_Counter'[o_c].fld_value = 0) \/ ((v_Counter'[o_c].fld_value <= 5) /\ m_T_h0[o_c]))]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ <>([](m_T_h0[o_c]))

===="#
    );
}

/// FSpec FR-813-AC-6: `eventually always` over a `since` as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-6")]
fn co_buchi_since_lean() {
    assert_eq!(
        clause_text(
            "eventually always (holds(c.value <= 5) since holds(c.value = 0))",
            Target::Lean
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter /\ ((0 <= 0) /\ (0 <= 9))

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value <= 5)

def at_T_1 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value = 0)

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (exists d1 : Int, 0 <= d1 /\ (forall d2 : Int, 0 <= d2 -> (exists d3 : Int, 0 <= d3 /\ 0 <= (((i + d1) + d2) - d3) /\ (0 <= (((i + d1) + d2) - d3) /\ at_T_1 (b.st (Int.toNat (((i + d1) + d2) - d3))) o_c) /\ (forall e4 : Int, 0 <= e4 -> e4 < d3 -> (0 <= (((i + d1) + d2) - e4) /\ at_T_0 (b.st (Int.toNat (((i + d1) + d2) - e4))) o_c)))))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-23: `once` as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-23")]
fn once_tla() {
    assert_eq!(
        clause_text(
            "always (holds(c.value >= 2) implies once holds(c.value = 1))",
            Target::Tla
        ),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter /\ ((0 <= 1) /\ (1 <= 9))

vars_T == <<vars, m_T_h0>>

TypeOK_T ==
  /\ m_T_h0 \in [Ids_Counter -> BOOLEAN]

Init_T ==
  /\ Init
  /\ m_T_h0 = [o_c \in Ids_Counter |-> (v_Counter[o_c].fld_value = 1)]
  /\ TypeOK_T

Mon_T ==
  /\ m_T_h0' = [o_c \in Ids_Counter |-> ((v_Counter'[o_c].fld_value = 1) \/ m_T_h0[o_c])]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ [](((v_Counter[o_c].fld_value >= 2) => m_T_h0[o_c]))

===="#
    );
}

/// FSpec FR-813-AC-23: `once` as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-23")]
fn once_lean() {
    assert_eq!(
        clause_text(
            "always (holds(c.value >= 2) implies once holds(c.value = 1))",
            Target::Lean
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter /\ ((0 <= 1) /\ (1 <= 9))

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value >= 2)

def at_T_1 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value = 1)

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 0 <= d1 -> ((0 <= (i + d1) /\ at_T_0 (b.st (Int.toNat (i + d1))) o_c) -> (exists d2 : Int, 0 <= d2 /\ 0 <= ((i + d1) - d2) /\ (0 <= ((i + d1) - d2) /\ at_T_1 (b.st (Int.toNat ((i + d1) - d2))) o_c))))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-7: `release` as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-7")]
fn release_tla() {
    assert_eq!(
        clause_text(
            "holds(c.value <= 4) release holds(c.value <= 5)",
            Target::Tla
        ),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter

vars_T == <<vars, m_T>>

TypeOK_T ==
  /\ m_T \in [Ids_Counter -> {"run", "rel", "rej"}]

Init_T ==
  /\ Init
  /\ m_T = [o_c \in Ids_Counter |-> IF ~(v_Counter[o_c].fld_value <= 5) THEN "rej" ELSE IF (v_Counter[o_c].fld_value <= 4) THEN "rel" ELSE "run"]
  /\ TypeOK_T

Mon_T ==
  /\ m_T' = [o_c \in Ids_Counter |-> IF m_T[o_c] /= "run" THEN m_T[o_c] ELSE IF ~(v_Counter'[o_c].fld_value <= 5) THEN "rej" ELSE IF (v_Counter'[o_c].fld_value <= 4) THEN "rel" ELSE "run"]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

accept_T(o_c) == m_T[o_c] /= "rej"

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ []accept_T(o_c)

===="#
    );
}

/// FSpec FR-813-AC-7: `release` as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-7")]
fn release_lean() {
    assert_eq!(
        clause_text(
            "holds(c.value <= 4) release holds(c.value <= 5)",
            Target::Lean
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value <= 4)

def at_T_1 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value <= 5)

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  Not (exists d1 : Int, 0 <= d1 /\ Not ((0 <= (i + d1) /\ at_T_1 (b.st (Int.toNat (i + d1))) o_c)) /\ (forall e2 : Int, 0 <= e2 -> e2 < d1 -> Not ((0 <= (i + e2) /\ at_T_0 (b.st (Int.toNat (i + e2))) o_c))))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-3: an `eventually` over steps 1 to 3 as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-3")]
fn interval_eventually_tla() {
    assert_eq!(
        clause_text("eventually[1, 3] holds(c.value = 2)", Target::Tla),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter /\ ((0 <= 2) /\ (2 <= 9))

vars_T == <<vars, n_T_0, m_T>>

TypeOK_T ==
  /\ n_T_0 \in 0..3
  /\ m_T \in [Ids_Counter -> {"run", "acc", "rej"}]

Init_T ==
  /\ Init
  /\ n_T_0 = 0
  /\ m_T = [o_c \in Ids_Counter |-> IF (1 <= 0 /\ 0 <= 3) /\ (v_Counter[o_c].fld_value = 2) THEN "acc" ELSE IF 0 >= 3 THEN "rej" ELSE "run"]
  /\ TypeOK_T

Mon_T ==
  /\ n_T_0' = IF n_T_0 < 3 THEN n_T_0 + 1 ELSE 3
  /\ m_T' = [o_c \in Ids_Counter |-> IF m_T[o_c] /= "run" THEN m_T[o_c] ELSE IF (1 <= n_T_0' /\ n_T_0' <= 3) /\ (v_Counter'[o_c].fld_value = 2) THEN "acc" ELSE IF n_T_0' >= 3 THEN "rej" ELSE "run"]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

accept_T(o_c) == m_T[o_c] = "acc"

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ []<>accept_T(o_c)

===="#
    );
}

/// FSpec FR-813-AC-3: an `eventually` over steps 1 to 3 as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-3")]
fn interval_eventually_lean() {
    assert_eq!(
        clause_text("eventually[1, 3] holds(c.value = 2)", Target::Lean),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter /\ ((0 <= 2) /\ (2 <= 9))

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value = 2)

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (exists d1 : Int, 1 <= d1 /\ d1 <= 3 /\ (0 <= (i + d1) /\ at_T_0 (b.st (Int.toNat (i + d1))) o_c))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-4: strong fairness as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-4")]
fn strong_fairness_tla() {
    assert_eq!(
        clause_text(
            "fair strong Counter::Counter::inc; always eventually holds(c.value = 2)",
            Target::Tla
        ),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter /\ ((0 <= 2) /\ (2 <= 9))

Spec_T == Init /\ [][Next]_vars /\ WF_vars(Next) /\ SF_vars(\E r_self \in Ids_Counter : op_Counter_inc(r_self))

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ [](<>((v_Counter[o_c].fld_value = 2)))

===="#
    );
}

/// FSpec FR-813-AC-4: strong fairness as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-4")]
fn strong_fairness_lean() {
    assert_eq!(
        clause_text(
            "fair strong Counter::Counter::inc; always eventually holds(c.value = 2)",
            Target::Lean
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter /\ ((0 <= 2) /\ (2 <= 9))

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value = 2)

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 0 <= d1 -> (exists d2 : Int, 0 <= d2 /\ (0 <= ((i + d1) + d2) /\ at_T_0 (b.st (Int.toNat ((i + d1) + d2))) o_c)))

def fair_T (b : Behaviour) : Prop :=
  ((forall i : Nat, exists j : Nat, i <= j /\ (exists t, exists r_self, op_Counter_inc (b.st j) t r_self)) -> (forall i : Nat, exists j : Nat, i <= j /\ (exists r_self, op_Counter_inc (b.st j) (b.st (j + 1)) r_self)))

def Ob_T : Prop :=
  forall b : Behaviour, fair_T b -> forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-22: weak `each` fairness as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-22")]
fn weak_each_fairness_tla() {
    assert_eq!(
        clause_text(
            "fair weak each Counter::Counter::inc; always eventually holds(c.value = 2)",
            Target::Tla
        ),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter /\ ((0 <= 2) /\ (2 <= 9))

Spec_T == Init /\ [][Next]_vars /\ WF_vars(Next) /\ \A r_self \in Ids_Counter : WF_vars(op_Counter_inc(r_self))

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ [](<>((v_Counter[o_c].fld_value = 2)))

===="#
    );
}

/// FSpec FR-813-AC-22: weak `each` fairness as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-22")]
fn weak_each_fairness_lean() {
    assert_eq!(
        clause_text(
            "fair weak each Counter::Counter::inc; always eventually holds(c.value = 2)",
            Target::Lean
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter /\ ((0 <= 2) /\ (2 <= 9))

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value = 2)

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 0 <= d1 -> (exists d2 : Int, 0 <= d2 /\ (0 <= ((i + d1) + d2) /\ at_T_0 (b.st (Int.toNat ((i + d1) + d2))) o_c)))

def fair_T (b : Behaviour) : Prop :=
  (forall r_self, Ids_Counter r_self -> (forall i : Nat, (forall j : Nat, i <= j -> (exists t, op_Counter_inc (b.st j) t r_self)) -> (exists j : Nat, i <= j /\ (op_Counter_inc (b.st j) (b.st (j + 1)) r_self))))

def Ob_T : Prop :=
  forall b : Behaviour, fair_T b -> forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-22: weak `each` fairness over a monitored clause as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-22")]
fn monitored_each_fairness_tla() {
    assert_eq!(
        clause_text(
            "fair weak each Counter::Counter::inc; always eventually (holds(c.value = 2) since holds(c.value = 1))",
            Target::Tla
        ),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter /\ ((0 <= 2) /\ (2 <= 9)) /\ ((0 <= 1) /\ (1 <= 9))

vars_T == <<vars, m_T_h0>>

TypeOK_T ==
  /\ m_T_h0 \in [Ids_Counter -> BOOLEAN]

Init_T ==
  /\ Init
  /\ m_T_h0 = [o_c \in Ids_Counter |-> (v_Counter[o_c].fld_value = 1)]
  /\ TypeOK_T

Mon_T ==
  /\ m_T_h0' = [o_c \in Ids_Counter |-> ((v_Counter'[o_c].fld_value = 1) \/ ((v_Counter'[o_c].fld_value = 2) /\ m_T_h0[o_c]))]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T) /\ \A r_self \in Ids_Counter : WF_vars(op_Counter_inc(r_self))

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ [](<>(m_T_h0[o_c]))

===="#
    );
}

/// FSpec FR-813-AC-22: weak `each` fairness over a monitored clause as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-22")]
fn monitored_each_fairness_lean() {
    assert_eq!(
        clause_text(
            "fair weak each Counter::Counter::inc; always eventually (holds(c.value = 2) since holds(c.value = 1))",
            Target::Lean
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter /\ ((0 <= 2) /\ (2 <= 9)) /\ ((0 <= 1) /\ (1 <= 9))

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value = 2)

def at_T_1 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value = 1)

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 0 <= d1 -> (exists d2 : Int, 0 <= d2 /\ (exists d3 : Int, 0 <= d3 /\ 0 <= (((i + d1) + d2) - d3) /\ (0 <= (((i + d1) + d2) - d3) /\ at_T_1 (b.st (Int.toNat (((i + d1) + d2) - d3))) o_c) /\ (forall e4 : Int, 0 <= e4 -> e4 < d3 -> (0 <= (((i + d1) + d2) - e4) /\ at_T_0 (b.st (Int.toNat (((i + d1) + d2) - e4))) o_c)))))

def fair_T (b : Behaviour) : Prop :=
  (forall r_self, Ids_Counter r_self -> (forall i : Nat, (forall j : Nat, i <= j -> (exists t, op_Counter_inc (b.st j) t r_self)) -> (exists j : Nat, i <= j /\ (op_Counter_inc (b.st j) (b.st (j + 1)) r_self))))

def Ob_T : Prop :=
  forall b : Behaviour, fair_T b -> forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-4: weak fairness without `each` as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-4")]
fn weak_fairness_tla() {
    assert_eq!(
        clause_text(
            "fair weak Counter::Counter::inc; always eventually holds(c.value = 2)",
            Target::Tla
        ),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter /\ ((0 <= 2) /\ (2 <= 9))

Spec_T == Init /\ [][Next]_vars /\ WF_vars(Next) /\ WF_vars(\E r_self \in Ids_Counter : op_Counter_inc(r_self))

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ [](<>((v_Counter[o_c].fld_value = 2)))

===="#
    );
}

/// FSpec FR-813-AC-4: weak fairness without `each` as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-4")]
fn weak_fairness_lean() {
    assert_eq!(
        clause_text(
            "fair weak Counter::Counter::inc; always eventually holds(c.value = 2)",
            Target::Lean
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter /\ ((0 <= 2) /\ (2 <= 9))

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value = 2)

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 0 <= d1 -> (exists d2 : Int, 0 <= d2 /\ (0 <= ((i + d1) + d2) /\ at_T_0 (b.st (Int.toNat ((i + d1) + d2))) o_c)))

def fair_T (b : Behaviour) : Prop :=
  (forall i : Nat, (forall j : Nat, i <= j -> (exists t, exists r_self, op_Counter_inc (b.st j) t r_self)) -> (exists j : Nat, i <= j /\ (exists r_self, op_Counter_inc (b.st j) (b.st (j + 1)) r_self)))

def Ob_T : Prop :=
  forall b : Behaviour, fair_T b -> forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-23: `historically` as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-23")]
fn historically_tla() {
    assert_eq!(
        clause_text(
            "always (holds(c.value = 5) implies historically holds(c.value >= 0))",
            Target::Tla
        ),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter /\ ((0 <= 5) /\ (5 <= 9))

vars_T == <<vars, m_T_h0>>

TypeOK_T ==
  /\ m_T_h0 \in [Ids_Counter -> BOOLEAN]

Init_T ==
  /\ Init
  /\ m_T_h0 = [o_c \in Ids_Counter |-> (v_Counter[o_c].fld_value >= 0)]
  /\ TypeOK_T

Mon_T ==
  /\ m_T_h0' = [o_c \in Ids_Counter |-> ((v_Counter'[o_c].fld_value >= 0) /\ m_T_h0[o_c])]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ [](((v_Counter[o_c].fld_value = 5) => m_T_h0[o_c]))

===="#
    );
}

/// FSpec FR-813-AC-23: `historically` as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-23")]
fn historically_lean() {
    assert_eq!(
        clause_text(
            "always (holds(c.value = 5) implies historically holds(c.value >= 0))",
            Target::Lean
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter /\ ((0 <= 5) /\ (5 <= 9))

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value = 5)

def at_T_1 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value >= 0)

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 0 <= d1 -> ((0 <= (i + d1) /\ at_T_0 (b.st (Int.toNat (i + d1))) o_c) -> (forall d2 : Int, 0 <= d2 -> 0 <= ((i + d1) - d2) -> (0 <= ((i + d1) - d2) /\ at_T_1 (b.st (Int.toNat ((i + d1) - d2))) o_c))))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-23: `historically[1, *]` as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-23")]
fn historically_from_tla() {
    assert_eq!(
        clause_text(
            "always (holds(c.value = 5) implies historically[1, *] holds(c.value = 1))",
            Target::Tla
        ),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter /\ ((0 <= 5) /\ (5 <= 9)) /\ ((0 <= 1) /\ (1 <= 9))

vars_T == <<vars, m_T_c0>>

TypeOK_T ==
  /\ m_T_c0 \in [Ids_Counter -> -1..1]

Init_T ==
  /\ Init
  /\ m_T_c0 = [o_c \in Ids_Counter |-> IF ~((v_Counter[o_c].fld_value = 1)) THEN 0 ELSE -1]
  /\ TypeOK_T

Mon_T ==
  /\ m_T_c0' = [o_c \in Ids_Counter |-> IF m_T_c0[o_c] = -1 THEN (IF ~((v_Counter'[o_c].fld_value = 1)) THEN 0 ELSE -1) ELSE IF m_T_c0[o_c] < 1 THEN m_T_c0[o_c] + 1 ELSE 1]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ [](((v_Counter[o_c].fld_value = 5) => ~(m_T_c0[o_c] = 1)))

===="#
    );
}

/// FSpec FR-813-AC-23: `historically[1, *]` as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-23")]
fn historically_from_lean() {
    assert_eq!(
        clause_text(
            "always (holds(c.value = 5) implies historically[1, *] holds(c.value = 1))",
            Target::Lean
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter /\ ((0 <= 5) /\ (5 <= 9)) /\ ((0 <= 1) /\ (1 <= 9))

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value = 5)

def at_T_1 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value = 1)

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 0 <= d1 -> ((0 <= (i + d1) /\ at_T_0 (b.st (Int.toNat (i + d1))) o_c) -> (forall d2 : Int, 1 <= d2 -> 0 <= ((i + d1) - d2) -> (0 <= ((i + d1) - d2) /\ at_T_1 (b.st (Int.toNat ((i + d1) - d2))) o_c))))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-23: `historically[1, 1]` as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-23")]
fn historically_closed_tla() {
    assert_eq!(
        clause_text(
            "always (holds(c.value = 5) implies historically[1, 1] holds(c.value = 1))",
            Target::Tla
        ),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter /\ ((0 <= 5) /\ (5 <= 9)) /\ ((0 <= 1) /\ (1 <= 9))

vars_T == <<vars, m_T_r0>>

TypeOK_T ==
  /\ m_T_r0 \in [Ids_Counter -> [1..1 -> BOOLEAN]]

Init_T ==
  /\ Init
  /\ m_T_r0 = [o_c \in Ids_Counter |-> [k \in 1..1 |-> TRUE]]
  /\ TypeOK_T

Mon_T ==
  /\ m_T_r0' = [o_c \in Ids_Counter |-> [k \in 1..1 |-> IF k = 1 THEN ~((v_Counter[o_c].fld_value = 1)) ELSE m_T_r0[o_c][k - 1]]]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ [](((v_Counter[o_c].fld_value = 5) => ~(((\E k \in 1..1 : m_T_r0[o_c][k])))))

===="#
    );
}

/// FSpec FR-813-AC-23: `historically[1, 1]` as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-23")]
fn historically_closed_lean() {
    assert_eq!(
        clause_text(
            "always (holds(c.value = 5) implies historically[1, 1] holds(c.value = 1))",
            Target::Lean
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter /\ ((0 <= 5) /\ (5 <= 9)) /\ ((0 <= 1) /\ (1 <= 9))

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value = 5)

def at_T_1 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value = 1)

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 0 <= d1 -> ((0 <= (i + d1) /\ at_T_0 (b.st (Int.toNat (i + d1))) o_c) -> (forall d2 : Int, 1 <= d2 -> d2 <= 1 -> (0 <= ((i + d1) - d2) /\ at_T_1 (b.st (Int.toNat ((i + d1) - d2))) o_c))))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-23: `since` as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-23")]
fn since_tla() {
    assert_eq!(
        clause_text(
            "always (holds(c.value = 2) implies (holds(c.value >= 1) since holds(c.value = 0)))",
            Target::Tla
        ),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter /\ ((0 <= 2) /\ (2 <= 9)) /\ ((0 <= 0) /\ (0 <= 9))

vars_T == <<vars, m_T_h0>>

TypeOK_T ==
  /\ m_T_h0 \in [Ids_Counter -> BOOLEAN]

Init_T ==
  /\ Init
  /\ m_T_h0 = [o_c \in Ids_Counter |-> (v_Counter[o_c].fld_value = 0)]
  /\ TypeOK_T

Mon_T ==
  /\ m_T_h0' = [o_c \in Ids_Counter |-> ((v_Counter'[o_c].fld_value = 0) \/ ((v_Counter'[o_c].fld_value >= 1) /\ m_T_h0[o_c]))]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ [](((v_Counter[o_c].fld_value = 2) => m_T_h0[o_c]))

===="#
    );
}

/// FSpec FR-813-AC-23: `since` as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-23")]
fn since_lean() {
    assert_eq!(
        clause_text(
            "always (holds(c.value = 2) implies (holds(c.value >= 1) since holds(c.value = 0)))",
            Target::Lean
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter /\ ((0 <= 2) /\ (2 <= 9)) /\ ((0 <= 0) /\ (0 <= 9))

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value = 2)

def at_T_1 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value >= 1)

def at_T_2 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value = 0)

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 0 <= d1 -> ((0 <= (i + d1) /\ at_T_0 (b.st (Int.toNat (i + d1))) o_c) -> (exists d2 : Int, 0 <= d2 /\ 0 <= ((i + d1) - d2) /\ (0 <= ((i + d1) - d2) /\ at_T_2 (b.st (Int.toNat ((i + d1) - d2))) o_c) /\ (forall e3 : Int, 0 <= e3 -> e3 < d2 -> (0 <= ((i + d1) - e3) /\ at_T_1 (b.st (Int.toNat ((i + d1) - e3))) o_c)))))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-10: `since[1, 2]` with two registers as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-10")]
fn since_within_tla() {
    assert_eq!(
        clause_text(
            "always (holds(c.value = 2) implies (holds(c.value >= 1) since[1, 2] holds(c.value = 0)))",
            Target::Tla
        ),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter /\ ((0 <= 2) /\ (2 <= 9)) /\ ((0 <= 0) /\ (0 <= 9))

vars_T == <<vars, m_T_r0, m_T_r1>>

TypeOK_T ==
  /\ m_T_r0 \in [Ids_Counter -> [1..2 -> BOOLEAN]]
  /\ m_T_r1 \in [Ids_Counter -> [1..2 -> BOOLEAN]]

Init_T ==
  /\ Init
  /\ m_T_r0 = [o_c \in Ids_Counter |-> [k \in 1..2 |-> FALSE]]
  /\ m_T_r1 = [o_c \in Ids_Counter |-> [k \in 1..2 |-> FALSE]]
  /\ TypeOK_T

Mon_T ==
  /\ m_T_r0' = [o_c \in Ids_Counter |-> [k \in 1..2 |-> IF k = 1 THEN (v_Counter[o_c].fld_value = 0) ELSE m_T_r0[o_c][k - 1]]]
  /\ m_T_r1' = [o_c \in Ids_Counter |-> [k \in 1..2 |-> IF k = 1 THEN (v_Counter[o_c].fld_value >= 1) ELSE m_T_r1[o_c][k - 1]]]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ [](((v_Counter[o_c].fld_value = 2) => (\E d \in 1..2 : (IF d = 0 THEN (v_Counter[o_c].fld_value = 0) ELSE m_T_r0[o_c][d]) /\ (\A e \in 1..(d - 1) : IF e = 0 THEN (v_Counter[o_c].fld_value >= 1) ELSE m_T_r1[o_c][e]))))

===="#
    );
}

/// FSpec FR-813-AC-10: `since[1, 2]` with two registers as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-10")]
fn since_within_lean() {
    assert_eq!(
        clause_text(
            "always (holds(c.value = 2) implies (holds(c.value >= 1) since[1, 2] holds(c.value = 0)))",
            Target::Lean
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter /\ ((0 <= 2) /\ (2 <= 9)) /\ ((0 <= 0) /\ (0 <= 9))

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value = 2)

def at_T_1 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value >= 1)

def at_T_2 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value = 0)

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 0 <= d1 -> ((0 <= (i + d1) /\ at_T_0 (b.st (Int.toNat (i + d1))) o_c) -> (exists d2 : Int, 1 <= d2 /\ d2 <= 2 /\ (0 <= ((i + d1) - d2) /\ at_T_2 (b.st (Int.toNat ((i + d1) - d2))) o_c) /\ (forall e3 : Int, 1 <= e3 -> e3 < d2 -> (0 <= ((i + d1) - e3) /\ at_T_1 (b.st (Int.toNat ((i + d1) - e3))) o_c)))))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-23: `triggered` as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-23")]
fn triggered_tla() {
    assert_eq!(
        clause_text(
            "always (holds(c.value = 2) implies (holds(c.value >= 0) triggered holds(c.value <= 5)))",
            Target::Tla
        ),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter /\ ((0 <= 2) /\ (2 <= 9))

vars_T == <<vars, m_T_h0>>

TypeOK_T ==
  /\ m_T_h0 \in [Ids_Counter -> BOOLEAN]

Init_T ==
  /\ Init
  /\ m_T_h0 = [o_c \in Ids_Counter |-> (v_Counter[o_c].fld_value <= 5)]
  /\ TypeOK_T

Mon_T ==
  /\ m_T_h0' = [o_c \in Ids_Counter |-> ((v_Counter'[o_c].fld_value <= 5) /\ ((v_Counter'[o_c].fld_value >= 0) \/ m_T_h0[o_c]))]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ [](((v_Counter[o_c].fld_value = 2) => m_T_h0[o_c]))

===="#
    );
}

/// FSpec FR-813-AC-23: `triggered` as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-23")]
fn triggered_lean() {
    assert_eq!(
        clause_text(
            "always (holds(c.value = 2) implies (holds(c.value >= 0) triggered holds(c.value <= 5)))",
            Target::Lean
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter /\ ((0 <= 2) /\ (2 <= 9))

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value = 2)

def at_T_1 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value >= 0)

def at_T_2 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value <= 5)

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 0 <= d1 -> ((0 <= (i + d1) /\ at_T_0 (b.st (Int.toNat (i + d1))) o_c) -> Not (exists d2 : Int, 0 <= d2 /\ 0 <= ((i + d1) - d2) /\ Not ((0 <= ((i + d1) - d2) /\ at_T_2 (b.st (Int.toNat ((i + d1) - d2))) o_c)) /\ (forall e3 : Int, 0 <= e3 -> e3 < d2 -> Not ((0 <= ((i + d1) - e3) /\ at_T_1 (b.st (Int.toNat ((i + d1) - e3))) o_c))))))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-7: `until` over steps 1 to 3 as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-7")]
fn bounded_until_tla() {
    assert_eq!(
        clause_text(
            "holds(c.value >= 1) until[1, 3] holds(c.value >= 3)",
            Target::Tla
        ),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter

vars_T == <<vars, n_T_0, m_T>>

TypeOK_T ==
  /\ n_T_0 \in 0..3
  /\ m_T \in [Ids_Counter -> {"run", "acc", "rej"}]

Init_T ==
  /\ Init
  /\ n_T_0 = 0
  /\ m_T = [o_c \in Ids_Counter |-> IF (1 <= 0 /\ 0 <= 3) /\ (v_Counter[o_c].fld_value >= 3) THEN "acc" ELSE IF (1 <= 0) /\ ~(v_Counter[o_c].fld_value >= 1) THEN "rej" ELSE IF 0 >= 3 THEN "rej" ELSE "run"]
  /\ TypeOK_T

Mon_T ==
  /\ n_T_0' = IF n_T_0 < 3 THEN n_T_0 + 1 ELSE 3
  /\ m_T' = [o_c \in Ids_Counter |-> IF m_T[o_c] /= "run" THEN m_T[o_c] ELSE IF (1 <= n_T_0' /\ n_T_0' <= 3) /\ (v_Counter'[o_c].fld_value >= 3) THEN "acc" ELSE IF (1 <= n_T_0') /\ ~(v_Counter'[o_c].fld_value >= 1) THEN "rej" ELSE IF n_T_0' >= 3 THEN "rej" ELSE "run"]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

accept_T(o_c) == m_T[o_c] = "acc"

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ []<>accept_T(o_c)

===="#
    );
}

/// FSpec FR-813-AC-7: `until` over steps 1 to 3 as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-7")]
fn bounded_until_lean() {
    assert_eq!(
        clause_text(
            "holds(c.value >= 1) until[1, 3] holds(c.value >= 3)",
            Target::Lean
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value >= 1)

def at_T_1 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value >= 3)

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (exists d1 : Int, 1 <= d1 /\ d1 <= 3 /\ (0 <= (i + d1) /\ at_T_1 (b.st (Int.toNat (i + d1))) o_c) /\ (forall e2 : Int, 1 <= e2 -> e2 < d1 -> (0 <= (i + e2) /\ at_T_0 (b.st (Int.toNat (i + e2))) o_c)))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-9: `once[2, *]` as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-9")]
fn once_from_tla() {
    assert_eq!(
        clause_text(
            "always (holds(c.value = 5) implies once[2, *] holds(c.value = 1))",
            Target::Tla
        ),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter /\ ((0 <= 5) /\ (5 <= 9)) /\ ((0 <= 1) /\ (1 <= 9))

vars_T == <<vars, m_T_c0>>

TypeOK_T ==
  /\ m_T_c0 \in [Ids_Counter -> -1..2]

Init_T ==
  /\ Init
  /\ m_T_c0 = [o_c \in Ids_Counter |-> IF (v_Counter[o_c].fld_value = 1) THEN 0 ELSE -1]
  /\ TypeOK_T

Mon_T ==
  /\ m_T_c0' = [o_c \in Ids_Counter |-> IF m_T_c0[o_c] = -1 THEN (IF (v_Counter'[o_c].fld_value = 1) THEN 0 ELSE -1) ELSE IF m_T_c0[o_c] < 2 THEN m_T_c0[o_c] + 1 ELSE 2]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ [](((v_Counter[o_c].fld_value = 5) => m_T_c0[o_c] = 2))

===="#
    );
}

/// FSpec FR-813-AC-9: `once[2, *]` as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-9")]
fn once_from_lean() {
    assert_eq!(
        clause_text(
            "always (holds(c.value = 5) implies once[2, *] holds(c.value = 1))",
            Target::Lean
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter /\ ((0 <= 5) /\ (5 <= 9)) /\ ((0 <= 1) /\ (1 <= 9))

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value = 5)

def at_T_1 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value = 1)

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 0 <= d1 -> ((0 <= (i + d1) /\ at_T_0 (b.st (Int.toNat (i + d1))) o_c) -> (exists d2 : Int, 2 <= d2 /\ 0 <= ((i + d1) - d2) /\ (0 <= ((i + d1) - d2) /\ at_T_1 (b.st (Int.toNat ((i + d1) - d2))) o_c))))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-10: `since[2, *]` as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-10")]
fn since_from_tla() {
    assert_eq!(
        clause_text(
            "always (holds(c.value = 5) implies (holds(c.value >= 1) since[2, *] holds(c.value = 1)))",
            Target::Tla
        ),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter /\ ((0 <= 5) /\ (5 <= 9)) /\ ((0 <= 1) /\ (1 <= 9))

vars_T == <<vars, m_T_h0, m_T_r0>>

TypeOK_T ==
  /\ m_T_h0 \in [Ids_Counter -> BOOLEAN]
  /\ m_T_r0 \in [Ids_Counter -> [1..2 -> BOOLEAN]]

Init_T ==
  /\ Init
  /\ m_T_h0 = [o_c \in Ids_Counter |-> (v_Counter[o_c].fld_value = 1)]
  /\ m_T_r0 = [o_c \in Ids_Counter |-> [k \in 1..2 |-> FALSE]]
  /\ TypeOK_T

Mon_T ==
  /\ m_T_h0' = [o_c \in Ids_Counter |-> ((v_Counter'[o_c].fld_value = 1) \/ ((v_Counter'[o_c].fld_value >= 1) /\ m_T_h0[o_c]))]
  /\ m_T_r0' = [o_c \in Ids_Counter |-> [k \in 1..2 |-> IF k = 1 THEN m_T_h0[o_c] ELSE m_T_r0[o_c][k - 1]]]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ [](((v_Counter[o_c].fld_value = 5) => m_T_r0[o_c][2]))

===="#
    );
}

/// FSpec FR-813-AC-10: `since[2, *]` as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-10")]
fn since_from_lean() {
    assert_eq!(
        clause_text(
            "always (holds(c.value = 5) implies (holds(c.value >= 1) since[2, *] holds(c.value = 1)))",
            Target::Lean
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter /\ ((0 <= 5) /\ (5 <= 9)) /\ ((0 <= 1) /\ (1 <= 9))

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value = 5)

def at_T_1 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value >= 1)

def at_T_2 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value = 1)

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 0 <= d1 -> ((0 <= (i + d1) /\ at_T_0 (b.st (Int.toNat (i + d1))) o_c) -> (exists d2 : Int, 2 <= d2 /\ 0 <= ((i + d1) - d2) /\ (0 <= ((i + d1) - d2) /\ at_T_2 (b.st (Int.toNat ((i + d1) - d2))) o_c) /\ (forall e3 : Int, 2 <= e3 -> e3 < d2 -> (0 <= ((i + d1) - e3) /\ at_T_1 (b.st (Int.toNat ((i + d1) - e3))) o_c)))))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-8: `once[1, 3]` as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-8")]
fn once_within_tla() {
    assert_eq!(
        clause_text(
            "always (holds(c.value = 5) implies once[1, 3] holds(c.value = 1))",
            Target::Tla
        ),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter /\ ((0 <= 5) /\ (5 <= 9)) /\ ((0 <= 1) /\ (1 <= 9))

vars_T == <<vars, m_T_r0>>

TypeOK_T ==
  /\ m_T_r0 \in [Ids_Counter -> [1..3 -> BOOLEAN]]

Init_T ==
  /\ Init
  /\ m_T_r0 = [o_c \in Ids_Counter |-> [k \in 1..3 |-> FALSE]]
  /\ TypeOK_T

Mon_T ==
  /\ m_T_r0' = [o_c \in Ids_Counter |-> [k \in 1..3 |-> IF k = 1 THEN (v_Counter[o_c].fld_value = 1) ELSE m_T_r0[o_c][k - 1]]]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ [](((v_Counter[o_c].fld_value = 5) => ((\E k \in 1..3 : m_T_r0[o_c][k]))))

===="#
    );
}

/// FSpec FR-813-AC-8: `once[1, 3]` as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-8")]
fn once_within_lean() {
    assert_eq!(
        clause_text(
            "always (holds(c.value = 5) implies once[1, 3] holds(c.value = 1))",
            Target::Lean
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter /\ ((0 <= 5) /\ (5 <= 9)) /\ ((0 <= 1) /\ (1 <= 9))

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value = 5)

def at_T_1 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value = 1)

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 0 <= d1 -> ((0 <= (i + d1) /\ at_T_0 (b.st (Int.toNat (i + d1))) o_c) -> (exists d2 : Int, 1 <= d2 /\ d2 <= 3 /\ (0 <= ((i + d1) - d2) /\ at_T_1 (b.st (Int.toNat ((i + d1) - d2))) o_c))))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-10: `once[1, 1]` over a `since` as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-10")]
fn nested_past_tla() {
    assert_eq!(
        clause_text(
            "always (holds(c.value = 2) implies once[1, 1] (holds(c.value = 1) since holds(c.value = 0)))",
            Target::Tla
        ),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter /\ ((0 <= 2) /\ (2 <= 9)) /\ ((0 <= 1) /\ (1 <= 9)) /\ ((0 <= 0) /\ (0 <= 9))

vars_T == <<vars, m_T_h1, m_T_r0>>

TypeOK_T ==
  /\ m_T_h1 \in [Ids_Counter -> BOOLEAN]
  /\ m_T_r0 \in [Ids_Counter -> [1..1 -> BOOLEAN]]

Init_T ==
  /\ Init
  /\ m_T_h1 = [o_c \in Ids_Counter |-> (v_Counter[o_c].fld_value = 0)]
  /\ m_T_r0 = [o_c \in Ids_Counter |-> [k \in 1..1 |-> FALSE]]
  /\ TypeOK_T

Mon_T ==
  /\ m_T_h1' = [o_c \in Ids_Counter |-> ((v_Counter'[o_c].fld_value = 0) \/ ((v_Counter'[o_c].fld_value = 1) /\ m_T_h1[o_c]))]
  /\ m_T_r0' = [o_c \in Ids_Counter |-> [k \in 1..1 |-> IF k = 1 THEN m_T_h1[o_c] ELSE m_T_r0[o_c][k - 1]]]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ [](((v_Counter[o_c].fld_value = 2) => ((\E k \in 1..1 : m_T_r0[o_c][k]))))

===="#
    );
}

/// FSpec FR-813-AC-10: `once[1, 1]` over a `since` as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-10")]
fn nested_past_lean() {
    assert_eq!(
        clause_text(
            "always (holds(c.value = 2) implies once[1, 1] (holds(c.value = 1) since holds(c.value = 0)))",
            Target::Lean
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter /\ ((0 <= 2) /\ (2 <= 9)) /\ ((0 <= 1) /\ (1 <= 9)) /\ ((0 <= 0) /\ (0 <= 9))

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value = 2)

def at_T_1 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value = 1)

def at_T_2 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value = 0)

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 0 <= d1 -> ((0 <= (i + d1) /\ at_T_0 (b.st (Int.toNat (i + d1))) o_c) -> (exists d2 : Int, 1 <= d2 /\ d2 <= 1 /\ (exists d3 : Int, 0 <= d3 /\ 0 <= (((i + d1) - d2) - d3) /\ (0 <= (((i + d1) - d2) - d3) /\ at_T_2 (b.st (Int.toNat (((i + d1) - d2) - d3))) o_c) /\ (forall e4 : Int, 0 <= e4 -> e4 < d3 -> (0 <= (((i + d1) - d2) - e4) /\ at_T_1 (b.st (Int.toNat (((i + d1) - d2) - e4))) o_c))))))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-11: `once[1, 1]` at a terminal state as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-11")]
fn terminal_once_tla() {
    assert_eq!(
        clause_text(
            "eventually (holds(c.value = 5) and once[1, 1] holds(c.value = 5))",
            Target::Tla
        ),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter /\ ((0 <= 5) /\ (5 <= 9))

vars_T == <<vars, m_T_r0>>

TypeOK_T ==
  /\ m_T_r0 \in [Ids_Counter -> [1..1 -> BOOLEAN]]

Init_T ==
  /\ Init
  /\ m_T_r0 = [o_c \in Ids_Counter |-> [k \in 1..1 |-> FALSE]]
  /\ TypeOK_T

Mon_T ==
  /\ m_T_r0' = [o_c \in Ids_Counter |-> [k \in 1..1 |-> IF k = 1 THEN (v_Counter[o_c].fld_value = 5) ELSE m_T_r0[o_c][k - 1]]]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ <>(((v_Counter[o_c].fld_value = 5) /\ ((\E k \in 1..1 : m_T_r0[o_c][k]))))

===="#
    );
}

/// FSpec FR-813-AC-11: `once[1, 1]` at a terminal state as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-11")]
fn terminal_once_lean() {
    assert_eq!(
        clause_text(
            "eventually (holds(c.value = 5) and once[1, 1] holds(c.value = 5))",
            Target::Lean
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter /\ ((0 <= 5) /\ (5 <= 9))

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value = 5)

def at_T_1 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value = 5)

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (exists d1 : Int, 0 <= d1 /\ ((0 <= (i + d1) /\ at_T_0 (b.st (Int.toNat (i + d1))) o_c) /\ (exists d2 : Int, 1 <= d2 /\ d2 <= 1 /\ (0 <= ((i + d1) - d2) /\ at_T_1 (b.st (Int.toNat ((i + d1) - d2))) o_c))))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-21: `always[2, 3]` from a snapshot at value 4 as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-21")]
fn terminal_always_tla() {
    assert_eq!(
        clause_text_from("always[2, 3] holds(c.value = 5)", Target::Tla, Some(4),),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter /\ ((0 <= 5) /\ (5 <= 9))

vars_T == <<vars, n_T_0, m_T>>

TypeOK_T ==
  /\ n_T_0 \in 0..3
  /\ m_T \in [Ids_Counter -> {"run", "rel", "rej"}]

Init_T ==
  /\ Init
  /\ n_T_0 = 0
  /\ m_T = [o_c \in Ids_Counter |-> IF (2 <= 0 /\ 0 <= 3) /\ ~(v_Counter[o_c].fld_value = 5) THEN "rej" ELSE IF 0 >= 3 THEN "rel" ELSE "run"]
  /\ TypeOK_T

Mon_T ==
  /\ n_T_0' = IF n_T_0 < 3 THEN n_T_0 + 1 ELSE 3
  /\ m_T' = [o_c \in Ids_Counter |-> IF m_T[o_c] /= "run" THEN m_T[o_c] ELSE IF (2 <= n_T_0' /\ n_T_0' <= 3) /\ ~(v_Counter'[o_c].fld_value = 5) THEN "rej" ELSE IF n_T_0' >= 3 THEN "rel" ELSE "run"]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

accept_T(o_c) == m_T[o_c] /= "rej"

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ []accept_T(o_c)

===="#
    );
}

/// FSpec FR-813-AC-21: `always[2, 3]` from a snapshot at value 4 as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-21")]
fn terminal_always_lean() {
    assert_eq!(
        clause_text_from("always[2, 3] holds(c.value = 5)", Target::Lean, Some(4),),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter /\ ((0 <= 5) /\ (5 <= 9))

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value = 5)

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 2 <= d1 -> d1 <= 3 -> (0 <= (i + d1) /\ at_T_0 (b.st (Int.toNat (i + d1))) o_c))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-21: `eventually[2, 3]` of the terminal value as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-21")]
fn terminal_eventually_tla() {
    assert_eq!(
        clause_text_from("eventually[2, 3] holds(c.value = 5)", Target::Tla, Some(4),),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter /\ ((0 <= 5) /\ (5 <= 9))

vars_T == <<vars, n_T_0, m_T>>

TypeOK_T ==
  /\ n_T_0 \in 0..3
  /\ m_T \in [Ids_Counter -> {"run", "acc", "rej"}]

Init_T ==
  /\ Init
  /\ n_T_0 = 0
  /\ m_T = [o_c \in Ids_Counter |-> IF (2 <= 0 /\ 0 <= 3) /\ (v_Counter[o_c].fld_value = 5) THEN "acc" ELSE IF 0 >= 3 THEN "rej" ELSE "run"]
  /\ TypeOK_T

Mon_T ==
  /\ n_T_0' = IF n_T_0 < 3 THEN n_T_0 + 1 ELSE 3
  /\ m_T' = [o_c \in Ids_Counter |-> IF m_T[o_c] /= "run" THEN m_T[o_c] ELSE IF (2 <= n_T_0' /\ n_T_0' <= 3) /\ (v_Counter'[o_c].fld_value = 5) THEN "acc" ELSE IF n_T_0' >= 3 THEN "rej" ELSE "run"]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

accept_T(o_c) == m_T[o_c] = "acc"

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ []<>accept_T(o_c)

===="#
    );
}

/// FSpec FR-813-AC-21: `eventually[2, 3]` of the terminal value as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-21")]
fn terminal_eventually_lean() {
    assert_eq!(
        clause_text_from("eventually[2, 3] holds(c.value = 5)", Target::Lean, Some(4),),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter /\ ((0 <= 5) /\ (5 <= 9))

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value = 5)

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (exists d1 : Int, 2 <= d1 /\ d1 <= 3 /\ (0 <= (i + d1) /\ at_T_0 (b.st (Int.toNat (i + d1))) o_c))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-21: `eventually[2, 3]` of a value that is left as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-21")]
fn terminal_eventually_missed_tla() {
    assert_eq!(
        clause_text_from("eventually[2, 3] holds(c.value = 4)", Target::Tla, Some(4),),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter /\ ((0 <= 4) /\ (4 <= 9))

vars_T == <<vars, n_T_0, m_T>>

TypeOK_T ==
  /\ n_T_0 \in 0..3
  /\ m_T \in [Ids_Counter -> {"run", "acc", "rej"}]

Init_T ==
  /\ Init
  /\ n_T_0 = 0
  /\ m_T = [o_c \in Ids_Counter |-> IF (2 <= 0 /\ 0 <= 3) /\ (v_Counter[o_c].fld_value = 4) THEN "acc" ELSE IF 0 >= 3 THEN "rej" ELSE "run"]
  /\ TypeOK_T

Mon_T ==
  /\ n_T_0' = IF n_T_0 < 3 THEN n_T_0 + 1 ELSE 3
  /\ m_T' = [o_c \in Ids_Counter |-> IF m_T[o_c] /= "run" THEN m_T[o_c] ELSE IF (2 <= n_T_0' /\ n_T_0' <= 3) /\ (v_Counter'[o_c].fld_value = 4) THEN "acc" ELSE IF n_T_0' >= 3 THEN "rej" ELSE "run"]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

accept_T(o_c) == m_T[o_c] = "acc"

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ []<>accept_T(o_c)

===="#
    );
}

/// FSpec FR-813-AC-21: `eventually[2, 3]` of a value that is left as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-21")]
fn terminal_eventually_missed_lean() {
    assert_eq!(
        clause_text_from("eventually[2, 3] holds(c.value = 4)", Target::Lean, Some(4),),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter /\ ((0 <= 4) /\ (4 <= 9))

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value = 4)

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (exists d1 : Int, 2 <= d1 /\ d1 <= 3 /\ (0 <= (i + d1) /\ at_T_0 (b.st (Int.toNat (i + d1))) o_c))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-6: a nested future operator as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-6")]
fn nested_future_lean() {
    assert_eq!(
        clause_text(
            "always (holds(c.value >= 1) until holds(c.value >= 3))",
            Target::Lean
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value >= 1)

def at_T_1 (s : State) (o_c : List Nat) : Prop :=
  ((F_get_Counter o_c s.v_Counter).fld_value >= 3)

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 0 <= d1 -> (exists d2 : Int, 0 <= d2 /\ (0 <= ((i + d1) + d2) /\ at_T_1 (b.st (Int.toNat ((i + d1) + d2))) o_c) /\ (forall e3 : Int, 0 <= e3 -> e3 < d2 -> (0 <= ((i + d1) + e3) /\ at_T_0 (b.st (Int.toNat ((i + d1) + e3))) o_c))))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-5: the Lean file states obligations as propositions and
/// has no `theorem`, proof term or `sorry`, for every operator.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-5")]
fn the_lean_text_has_no_theorem_proof_or_sorry() {
    for body in [
        "always holds(c.value <= 5)",
        "holds(c.value >= 1) until holds(c.value >= 3)",
        "always (holds(c.value >= 2) implies once holds(c.value = 1))",
        "always (holds(c.value >= 2) implies historically holds(c.value >= 0))",
        "holds(c.value <= 4) release holds(c.value <= 5)",
        "fair weak each Counter::Counter::inc; eventually holds(c.value = 2)",
    ] {
        let text = exported_text(body, Target::Lean);
        for word in ["theorem", "THEOREM", "sorry", ":= by", "by simp"] {
            assert!(!text.contains(word), "{word}: {text}");
        }
        assert!(text.contains("def Ob_T : Prop :="), "{text}");
        assert!(text.is_ascii(), "{text}");
    }
}

/// FSpec FR-813-AC-1/AC-2/AC-6 and FR-811: each deterministic clause is
/// `preserved` with no cause and claims regions of the text that state it.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-1")]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-2")]
fn a_lowered_temporal_clause_is_preserved_and_claims_its_regions() {
    for target in [Target::Tla, Target::Lean] {
        for body in [
            "always holds(c.value <= 5)",
            "holds(c.value >= 1) until holds(c.value >= 3)",
            "eventually always holds(c.value <= 5)",
            "always[2, 3] holds(c.value = 5)",
        ] {
            let (exit, value) = export(body, target);
            assert_eq!(exit, 0, "{value}");
            let record = &value["export"]["records"][0];
            assert_eq!(record["disposition"], "preserved", "{value}");
            assert_eq!(record["causes"], json!([]), "{value}");
            let text = value["export"]["target_text"].as_str().unwrap();
            let regions = record["regions"].as_array().expect("regions");
            assert!(!regions.is_empty(), "{value}");
            for region in regions {
                let start = usize::try_from(region[0].as_u64().unwrap()).unwrap();
                let end = usize::try_from(region[1].as_u64().unwrap()).unwrap();
                let region = &text[start..end];
                assert!(region.contains('T') || region.contains("Ids_"), "{region}");
            }
        }
    }
}

/// FSpec FR-813-AC-3: the step counter of `always[0, 3]` is bounded by the
/// interval's upper bound, and the monitor advances on the terminal
/// stuttering step (`Next_T` stutters where `Next` is disabled), so inserted
/// stuttering steps leave the verdict unchanged.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-3")]
fn the_step_counter_is_bounded_and_stuttering_keeps_the_verdict() {
    let text = clause_text("always[0, 3] holds(c.value <= 5)", Target::Tla);
    assert!(text.contains("n_T_0 \\in 0..3"), "{text}");
    assert!(
        text.contains("Next_T == (Next \\/ (~ENABLED Next /\\ UNCHANGED vars)) /\\ Mon_T"),
        "{text}"
    );
    assert!(
        text.contains("n_T_0' = IF n_T_0 < 3 THEN n_T_0 + 1 ELSE 3"),
        "{text}"
    );
}

/// FSpec FR-813-AC-24 and FR-817-AC-16: a clause under a step-counting
/// profile other than `filament.temporal.infinite-trace/v1` is the typed
/// `unimplemented_capability` refusal naming the construct, exit 21, with a
/// node and a locus, never exit 30.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-24")]
fn a_clause_under_another_profile_is_an_unimplemented_capability() {
    for target in [Target::Tla, Target::Lean] {
        let (exit, value) = export_full(
            "always[0, 3] holds(c.value <= 5)",
            target,
            None,
            "filament.temporal.event-position.false-extension/v1",
            true,
        );
        assert_eq!(exit, 21, "{value}");
        let refusal = &value["export"]["refusal"];
        assert_eq!(refusal["code"], "unimplemented_capability", "{value}");
        assert_eq!(refusal["cause"], "unimplemented-meaning", "{value}");
        assert_eq!(refusal["construct"], "temporal/temporal_clause", "{value}");
        let family = if matches!(target, Target::Tla) {
            "tla"
        } else {
            "lean"
        };
        assert_eq!(
            refusal["capability"],
            format!("export:{family}:temporal/temporal_clause"),
            "{value}"
        );
        assert!(
            refusal["node"].is_string() && refusal["locus"].is_object(),
            "{value}"
        );
        assert_eq!(value["export"]["status"], "refused", "{value}");
    }
}

/// FSpec FR-813-AC-1/AC-7/AC-22: the identity universe is the snapshot's key
/// set, and no temporal definition holds `DOMAIN v_` outside `ok_T`.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-7")]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-22")]
fn the_identity_universe_is_the_snapshot_key_set() {
    let text = exported_text("always holds(c.value <= 5)", Target::Tla);
    assert!(text.contains("Ids_Counter == {<<99>>}"), "{text}");
    assert!(!text.contains("CONSTANT"), "{text}");
    for body in [
        "holds(c.value >= 1) until holds(c.value >= 3)",
        "always (holds(c.value >= 2) implies once[1, 2] holds(c.value = 1))",
        "fair weak each Counter::Counter::inc; always eventually holds(c.value = 2)",
    ] {
        let clause = clause_text(body, Target::Tla);
        let outside: Vec<&str> = clause
            .lines()
            .filter(|line| line.contains("DOMAIN v_") && !line.starts_with("ok_T("))
            .collect();
        assert!(outside.is_empty(), "{outside:?}");
    }
}

/// FSpec FR-813-AC-7: a request that fixes no keys exports the constant
/// `Keys_Counter` with its assumption and `TypeOK` bound, its TLA+ record is
/// `conditional` with exactly the `bounded-identity-universe` condition, its
/// Lean record stays `preserved`, and `Seq(Nat)` is no quantifier range.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-7")]
fn an_unfixed_key_set_is_a_constant_and_a_conditional_record() {
    let body = "holds(c.value >= 1) until holds(c.value >= 3)";
    let (exit, value) = export_unfixed(body, Target::Tla);
    assert_eq!(exit, 0, "{value}");
    let text = value["export"]["target_text"].as_str().unwrap();
    assert!(text.contains("CONSTANTS Keys_Counter\n"), "{text}");
    assert!(
        text.contains("ASSUME Keys_Counter \\subseteq Seq(Nat) /\\ IsFiniteSet(Keys_Counter)"),
        "{text}"
    );
    assert!(
        text.contains("DOMAIN v_Counter \\subseteq Keys_Counter"),
        "{text}"
    );
    assert!(text.contains("Ids_Counter == Keys_Counter"), "{text}");
    for line in text.lines() {
        assert!(
            !(line.contains("\\in Seq(Nat)") || line.contains("[Seq(Nat) ->")),
            "{line}"
        );
    }
    let record = &value["export"]["records"][0];
    assert_eq!(record["disposition"], "conditional", "{value}");
    let conditions = record["conditions"].as_array().unwrap();
    assert_eq!(conditions.len(), 1, "{value}");
    assert_eq!(
        conditions[0]["type"], "bounded-identity-universe",
        "{value}"
    );
    assert_eq!(conditions[0]["type_name"], "Counter", "{value}");
    assert!(conditions[0]["node"].is_string(), "{value}");
    assert_eq!(conditions[0].as_object().unwrap().len(), 3, "{value}");
    let (exit, value) = export_unfixed(body, Target::Lean);
    assert_eq!(exit, 0, "{value}");
    assert_eq!(
        value["export"]["records"][0]["disposition"], "preserved",
        "{value}"
    );
}

/// The refusal of `tail` under `target`: asserts the typed shape (exit 21,
/// `unimplemented_capability`, a node and a locus) and returns it.
fn refusal_of(tail: &str, target: Target) -> Value {
    let (exit, value) = export_tail(tail, target);
    assert_eq!(exit, 21, "{value}");
    assert_eq!(value["export"]["status"], "refused", "{value}");
    let refusal = value["export"]["refusal"].clone();
    assert_eq!(refusal["code"], "unimplemented_capability", "{value}");
    assert_eq!(refusal["cause"], "unimplemented-meaning", "{value}");
    assert!(
        refusal["node"].is_string() && refusal["locus"].is_object(),
        "{value}"
    );
    let family = if matches!(target, Target::Tla) {
        "tla"
    } else {
        "lean"
    };
    assert_eq!(
        refusal["capability"],
        format!("export:{family}:{}", refusal["construct"].as_str().unwrap()),
        "{value}"
    );
    refusal
}

fn clause_tail(body: &str) -> String {
    format!("{HEAD} {body} }}\n")
}

/// Future clauses that have a deterministic monitor can still be refused
/// while that lowering is missing.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-24")]
fn a_nested_future_operator_is_refused_in_tla_and_defined_in_lean() {
    for body in [
        "always (holds(c.value >= 1) until holds(c.value >= 3))",
        "always (holds(c.value = 1) implies eventually[1, 1] holds(c.value = 3))",
    ] {
        let refusal = refusal_of(&clause_tail(body), Target::Tla);
        assert_eq!(refusal["construct"], "temporal/formula");
        let (exit, value) = export_tail(&clause_tail(body), Target::Lean);
        assert_eq!(exit, 0, "{value}");
        let text = value["export"]["target_text"].as_str().unwrap();
        assert!(text.contains("def holds_T"), "{text}");
    }
}

fn tableau_export(limit: u64, target: Target) -> (u8, Value) {
    let dir = tempfile::tempdir().unwrap();
    let path = counter_request(
        dir.path(),
        json!({"profile": target.profile(), "obligations": ["T"]}),
    );
    let model_path = dir.path().join("counter.json");
    let model = std::fs::read_to_string(&model_path)
        .unwrap()
        .replace("Counter", "Lamp");
    let mut model: Value = serde_json::from_str(&model).unwrap();
    let object = &mut model["types"][1];
    let fields = object["fields"].as_array_mut().unwrap();
    let template = fields[0].clone();
    let snapshot_path = dir.path().join("counter-0.json");
    let mut snapshot: Value =
        serde_json::from_slice(&std::fs::read(&snapshot_path).unwrap()).unwrap();
    let row = &mut snapshot["populations"][0]["objects"][0];
    let identity = "ix://example/counter-third/Lamp";
    row["type"] = json!(identity);
    let values = &mut row["fields"];
    for name in ["safe", "seen", "pending", "ready"] {
        let mut field = template.clone();
        field["identity"] = json!(format!("{identity}/{name}"));
        field["name"] = json!(name);
        field["typeRef"] = json!("ix://filament/native/Boolean");
        fields.push(field);
        values[name] = json!({"boolean": false});
    }
    let mut light = object["operations"][0].clone();
    light["identity"] = json!(format!("{identity}/light"));
    light["name"] = json!("light");
    light["frame"]["modifies"] = json!([
        format!("{identity}/safe"),
        format!("{identity}/seen"),
        format!("{identity}/pending"),
    ]);
    let mut noop = light.clone();
    noop["identity"] = json!(format!("{identity}/noop"));
    noop["name"] = json!("noop");
    noop["frame"]["modifies"] = json!([]);
    object["operations"] = json!([light, noop]);
    std::fs::write(model_path, model.to_string()).unwrap();
    std::fs::write(snapshot_path, snapshot.to_string()).unwrap();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         profile t = \"filament.temporal.infinite-trace/v1\";\n\
         model Lamp = \"example/counter-third\";\n\
         pre CanLight using v on Lamp::Lamp::light {{ not self.safe }}\n\
         post Light using v on Lamp::Lamp::light {{ self.safe and self.seen and not self.pending }}\n\
         post Keep using v on Lamp::Lamp::noop {{ true }}\n\
         temporal T using t over (l: Lamp::Lamp) clock \"model-steps\" on origin {{\n\
           (eventually always holds(l.safe)) and (always eventually holds(l.seen)) and\n\
           (always (holds(l.pending) implies eventually[{limit}, {limit}] holds(l.ready)))\n\
         }}\n"
    );
    std::fs::write(dir.path().join("unit.native"), unit).unwrap();
    match fsl_command::command::run(&path, &crate::negotiation::Fr290) {
        Ok(result) => (
            result.exit_code,
            serde_json::from_str(&result.value.to_string()).unwrap(),
        ),
        Err(error) => (error.category().exit_code(), error.value().unwrap()),
    }
}

/// FR-813-AC-6: the mixed acceptance clause checks for an accepting run
/// of its negation without letting a bad guess block the model steps.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-6")]
fn tableau_checks_mixed_acceptance_over_the_total_product() {
    let (exit, tla) = tableau_export(1, Target::Tla);
    assert_eq!(exit, 0, "{tla}");
    let text = tla["export"]["target_text"].as_str().unwrap();
    for fragment in [
        "m_T \\in [Ids_Lamp -> F_bits_T]",
        "m_T' \\in [Ids_Lamp -> F_bits_T]",
        "F_check_T(o_l, q, r) ==",
        "m_T_h0' = [o_l \\in Ids_Lamp |->",
        "m_T_h0[o_l] /\\ F_check_T(o_l, m_T[o_l], m_T'[o_l])",
        "m_T_h1' = ~m_T_h1",
        "F_progress_T(m_T_c0[o_l], m_T[o_l])",
        "Next_T == (Next \\/ (~ENABLED Next /\\ UNCHANGED vars)) /\\ Mon_T",
        "WF_vars_T(Next_T)",
        "~([]<>accept_T(o_l))",
    ] {
        assert!(text.contains(fragment), "missing {fragment}: {text}");
    }
    let record = &tla["export"]["records"][0];
    assert_eq!(record["disposition"], "conditional", "{record}");
    let [condition] = record["conditions"].as_array().unwrap().as_slice() else {
        panic!("one condition: {record}");
    };
    assert_eq!(condition["type"], "nondeterministic-monitor", "{condition}");
    assert_eq!(condition["clause"], "T", "{condition}");
    let names = tla["export"]["name_map"].as_array().unwrap();
    for name in [
        "m_T",
        "m_T_h0",
        "m_T_h1",
        "m_T_c0",
        "F_bits_T",
        "F_check_T",
        "F_progress_T",
    ] {
        let matches: Vec<_> = names.iter().filter(|entry| entry["name"] == name).collect();
        let [entry] = matches.as_slice() else {
            panic!("expected one name map entry for {name}: {names:?}");
        };
        assert_eq!(condition["node"], entry["node"], "{entry}");
        assert!(entry.get("binding").is_none(), "{entry}");
    }
    let (exit, lean) = tableau_export(1, Target::Lean);
    assert_eq!(exit, 0, "{lean}");
    let text = lean["export"]["target_text"].as_str().unwrap();
    assert!(text.contains("def holds_T"), "{text}");
    assert_eq!(
        lean["export"]["records"][0]["disposition"], "preserved",
        "{lean}"
    );
}

#[test]
#[trace("FSpec-FR-812-AC-1", "FSpec-FR-813-AC-6", "FSpec-TC-1972")]
fn tableau_keeps_source_fairness_when_lamp_has_a_noop() {
    let (exit, tla) = tableau_export(1, Target::Tla);
    assert_eq!(exit, 0, "{tla}");
    let text = tla["export"]["target_text"].as_str().unwrap();
    let body = |name| {
        text.split_once(name)
            .unwrap()
            .1
            .split("\n\n")
            .next()
            .unwrap()
    };
    assert!(text.lines().any(|line| line == "vars == <<v_Lamp>>"));
    let init = body("Init ==\n");
    for field in ["pending", "safe", "seen"] {
        assert!(init.contains(&format!("fld_{field} |-> FALSE")), "{init}");
    }
    let noop = body("op_noop(r_self) ==\n");
    for field in ["pending", "ready", "safe", "seen", "value"] {
        assert!(
            noop.contains(&format!("v_Lamp'[k].fld_{field} = v_Lamp[k].fld_{field}")),
            "{noop}"
        );
    }
    let light = body("op_light(r_self) ==\n");
    for constraint in [
        "~v_Lamp[r_self].fld_safe",
        "v_Lamp'[r_self].fld_safe",
        "v_Lamp'[r_self].fld_seen",
        "~v_Lamp'[r_self].fld_pending",
    ] {
        assert!(light.contains(constraint), "missing {constraint}: {light}");
    }
    let next = body("Next ==\n");
    assert!(next.contains("op_noop(r_self)"), "{next}");
    assert!(next.contains("op_light(r_self)"), "{next}");
    let spec = r"Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T) /\ WF_vars(Next)";
    assert!(text.lines().any(|line| line == spec), "{text}");
}

#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-24")]
fn tableau_above_its_finite_closure_limit_has_a_typed_refusal() {
    let (exit, tla) = tableau_export(18, Target::Tla);
    assert_eq!(exit, 21, "{tla}");
    let refusal = &tla["export"]["refusal"];
    assert_eq!(refusal["code"], "unimplemented_capability", "{refusal}");
    assert_eq!(refusal["cause"], "unimplemented-meaning", "{refusal}");
    assert_eq!(refusal["construct"], "temporal/formula", "{refusal}");
    assert!(
        refusal["node"].is_string() && refusal["locus"].is_object(),
        "{refusal}"
    );
    let (exit, lean) = tableau_export(18, Target::Lean);
    assert_eq!(exit, 0, "{lean}");
    assert_eq!(
        lean["export"]["records"][0]["disposition"], "preserved",
        "{lean}"
    );
}

/// FSpec FR-813-AC-24: a future operator inside a past operator is refused in
/// both targets at the future operator's node.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-24")]
fn a_future_operator_inside_a_past_operand_is_refused() {
    for target in [Target::Tla, Target::Lean] {
        let refusal = refusal_of(
            &clause_tail("always once eventually holds(c.value = 1)"),
            target,
        );
        assert_eq!(refusal["construct"], "temporal/formula");
    }
}

/// FSpec FR-813-AC-24: an `on each` activation and a capture are refused at
/// their own nodes, never at the clause.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-24")]
fn an_activation_and_a_capture_are_refused_at_their_own_nodes() {
    let each = "temporal T using t over (c: Counter::Counter) clock \"model-steps\" on each (q: Counter::Counter) { always holds(c.value <= 5) }\n";
    let capture =
        clause_tail("capture big: Boolean = c.value > 3; always holds(c.value <= 5 or big)");
    for target in [Target::Tla, Target::Lean] {
        for tail in [each.to_owned(), capture.clone()] {
            let refusal = refusal_of(&tail, target);
            assert_ne!(refusal["construct"], "temporal/temporal_clause");
        }
    }
}

/// FSpec FR-813-AC-24: a clause over a subject that is no object type is
/// refused at its binder's node.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-24")]
fn a_non_object_subject_is_refused_at_its_binder() {
    let tail = "temporal T using t over (n: Int[0, 2]) clock \"steps\" on origin { always holds(n <= 2) }\n";
    for target in [Target::Tla, Target::Lean] {
        refusal_of(tail, target);
    }
}

/// FR-1015-AC-15 and FSpec FR-813-AC-24: even a valid origin protocol binding
/// remains a typed refusal while this mapper does not lower protocol subjects.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-24", "FR-1015-AC-15")]
fn a_valid_origin_protocol_subject_is_unimplemented_under_both_fidelity_floors() {
    let tail = format!(
        "{}protocol Hand using P over (k: Counter::Counter) on origin {{\n\
             role a on Counter::Counter;\n\
             requires temporal T;\n\
             run sequence Main {{ check Ok using v {{ true }}; }}\n\
             finish Done as (d: Counter::Counter) {{ true }};\n\
         }}\n",
        clause_tail("always holds(c.value <= 5)")
    );
    let (expected_node, expected_locus) = checked_protocol_identity_and_locus(&tail);
    for target in [Target::Tla, Target::Lean] {
        let family = match target {
            Target::Tla => "tla",
            Target::Lean => "lean",
        };
        for fidelity in [None, Some("any")] {
            let (exit, value) = export_unit_with_protocol_options(
                &tail,
                target,
                None,
                "filament.temporal.infinite-trace/v1",
                true,
                true,
                true,
                "example/counter-third",
                "c",
                json!({}),
                fidelity,
                Some("should-not-exist.export"),
            );
            assert_eq!(exit, 21, "{value}");
            assert_eq!(value["export"]["status"], "refused", "{value}");
            assert!(value["export"].get("target_text").is_none(), "{value}");
            assert!(value["export"].get("package").is_none(), "{value}");
            let refusal = &value["export"]["refusal"];
            assert_eq!(refusal["code"], "unimplemented_capability", "{value}");
            assert_eq!(refusal["cause"], "unimplemented-meaning", "{value}");
            assert_eq!(
                refusal["capability"],
                format!("export:{family}:protocol/protocol_clause"),
                "{value}"
            );
            assert_eq!(refusal["target"], family, "{value}");
            assert_eq!(refusal["construct"], "protocol/protocol_clause", "{value}");
            assert_eq!(refusal["node"], expected_node, "{value}");
            assert_eq!(refusal["locus"], expected_locus, "{value}");
            let fields: std::collections::BTreeSet<_> = refusal
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect();
            assert_eq!(
                fields,
                [
                    "capability",
                    "cause",
                    "code",
                    "construct",
                    "locus",
                    "node",
                    "target"
                ]
                .into_iter()
                .collect(),
                "{value}"
            );
        }
    }
}

#[test]
#[trace("FSpec-TC-1972", "FR-1015-AC-15")]
fn a_protocol_refusal_survives_an_omitted_initial_snapshot() {
    let tail = format!(
        "{}protocol Hand using P over (k: Counter::Counter) on origin {{\n\
             role a on Counter::Counter;\n\
             requires temporal T;\n\
             run sequence Main {{ check Ok using v {{ true }}; }}\n\
             finish Done as (d: Counter::Counter) {{ true }};\n\
         }}\n",
        clause_tail("always holds(c.value <= 5)")
    );
    let (exit, value) = export_unit_with_protocol_options(
        &tail,
        Target::Tla,
        None,
        "filament.temporal.infinite-trace/v1",
        false,
        true,
        true,
        "example/counter-third",
        "c",
        json!({}),
        None,
        None,
    );
    assert_eq!(exit, 21, "{value}");
    assert_eq!(
        value["export"]["refusal"]["construct"],
        "protocol/protocol_clause"
    );
}

#[test]
#[trace("FSpec-FR-813-AC-17")]
fn a_protocol_subject_with_an_unbound_static_role_refuses_wrong_role_mapping() {
    let tail = format!(
        "{}protocol Hand using P over (k: Counter::Counter) on origin {{\n\
             role a on Counter::Counter;\n\
             requires temporal T;\n\
             run sequence Main {{ check Ok using v {{ true }}; }}\n\
             finish Done as (d: Counter::Counter) {{ true }};\n\
         }}\n",
        clause_tail("always holds(c.value <= 5)")
    );
    let (exit, value) = export_tail_with_unbound_static_role(&tail, Target::Tla);
    assert_eq!(exit, 20, "{value}");
    assert_eq!(value["code"], "invalid_runtime_input", "{value}");
    assert_eq!(value["cause"], "wrong-role-mapping", "{value}");
    assert_eq!(
        value["details"]["path"], "/request/export/protocols/0/roles/a",
        "{value}"
    );
}

#[test]
#[trace("FSpec-FR-813-AC-24")]
fn an_unbound_source_owned_role_without_initial_refuses_with_typed_role_error() {
    let tail = format!(
        "object Order identity (total) {{ total: Int[0, 9]; population orders; }}\n\
         {}protocol Hand using P over (k: Counter::Counter) on origin {{\n\
             role Seller on Order;\n\
             requires temporal T;\n\
             run sequence Main {{ check Ok using v {{ true }}; }}\n\
             finish Done as (d: Counter::Counter) {{ true }};\n\
         }}\n",
        clause_tail("always holds(c.value <= 5)")
    );
    let (exit, value) = export_unit(
        &tail,
        Target::Tla,
        None,
        "filament.temporal.infinite-trace/v1",
        false,
    );

    assert_eq!(exit, 20, "{value}");
    assert_eq!(value["code"], "invalid_runtime_input", "{value}");
    assert_eq!(value["cause"], "wrong-role-mapping", "{value}");
    assert_eq!(
        value["details"]["path"], "/request/export/protocols/0/roles/Seller",
        "{value}"
    );
}

#[test]
#[trace("FSpec-FR-813-AC-24")]
fn a_protocol_static_role_binding_to_another_model_refuses_wrong_role_mapping() {
    let tail = format!(
        "{}protocol Hand using P over (k: Counter::Counter) on origin {{\n\
             role a on Counter::Counter;\n\
             requires temporal T;\n\
             run sequence Main {{ check Ok using v {{ true }}; }}\n\
             finish Done as (d: Counter::Counter) {{ true }};\n\
         }}\n",
        clause_tail("always holds(c.value <= 5)")
    );
    let (exit, value) = export_unit_with_protocol_options(
        &tail,
        Target::Tla,
        None,
        "filament.temporal.infinite-trace/v1",
        true,
        true,
        true,
        "example/another-model",
        "c",
        json!({}),
        None,
        None,
    );
    assert_eq!(exit, 20, "{value}");
    assert_eq!(value["code"], "invalid_runtime_input", "{value}");
    assert_eq!(value["cause"], "wrong-role-mapping", "{value}");
    assert_eq!(
        value["details"]["path"], "/request/export/protocols/0/roles/a",
        "{value}"
    );
}

#[test]
#[trace("FSpec-FR-813-AC-24")]
fn a_protocol_over_key_absent_from_the_initial_snapshot_refuses_wrong_role_mapping() {
    let tail = format!(
        "{}protocol Hand using P over (k: Counter::Counter) on origin {{\n\
             role a on Counter::Counter;\n\
             requires temporal T;\n\
             run sequence Main {{ check Ok using v {{ true }}; }}\n\
             finish Done as (d: Counter::Counter) {{ true }};\n\
         }}\n",
        clause_tail("always holds(c.value <= 5)")
    );
    let (exit, value) = export_unit_with_protocol_options(
        &tail,
        Target::Tla,
        None,
        "filament.temporal.infinite-trace/v1",
        true,
        true,
        true,
        "example/counter-third",
        "absent",
        json!({}),
        None,
        None,
    );
    assert_eq!(exit, 20, "{value}");
    assert_eq!(value["code"], "invalid_runtime_input", "{value}");
    assert_eq!(value["cause"], "wrong-role-mapping", "{value}");
    assert_eq!(
        value["details"]["path"], "/request/export/protocols/0/over",
        "{value}"
    );
}

#[test]
#[trace("FSpec-FR-813-AC-24")]
fn a_protocol_whole_model_role_is_auto_bound_and_rejects_a_request_binding() {
    let tail = format!(
        "{}protocol Hand using P over (k: Counter::Counter) on origin {{\n\
             role a on Counter;\n\
             requires temporal T;\n\
             run sequence Main {{ check Ok using v {{ true }}; }}\n\
             finish Done as (d: Counter::Counter) {{ true }};\n\
         }}\n",
        clause_tail("always holds(c.value <= 5)")
    );
    let (exit, value) = export_unit_with_protocol_options(
        &tail,
        Target::Tla,
        None,
        "filament.temporal.infinite-trace/v1",
        true,
        true,
        true,
        "example/counter-third",
        "c",
        json!({}),
        None,
        None,
    );
    assert_eq!(exit, 20, "{value}");
    assert_eq!(value["code"], "invalid_runtime_input", "{value}");
    assert_eq!(value["cause"], "wrong-role-mapping", "{value}");
    assert_eq!(
        value["details"]["path"], "/request/export/protocols/0/roles/a",
        "{value}"
    );
    assert!(
        value["message"]
            .as_str()
            .unwrap_or_default()
            .contains("auto-bound"),
        "{value}"
    );
}

#[test]
#[trace("FSpec-FR-813-AC-24", "FR-1015-AC-15")]
fn an_omitted_whole_model_role_binding_reaches_the_protocol_refusal() {
    let tail = format!(
        "{}protocol Hand using P over (k: Counter::Counter) on origin {{\n\
             role a on Counter;\n\
             requires temporal T;\n\
             run sequence Main {{ check Ok using v {{ true }}; }}\n\
             finish Done as (d: Counter::Counter) {{ true }};\n\
         }}\n",
        clause_tail("always holds(c.value <= 5)")
    );
    let (exit, value) = export_unit_with_protocol_options(
        &tail,
        Target::Tla,
        None,
        "filament.temporal.infinite-trace/v1",
        true,
        true,
        false,
        "example/counter-third",
        "c",
        json!({}),
        None,
        None,
    );
    assert_eq!(exit, 21, "{value}");
    assert_eq!(
        value["export"]["refusal"]["construct"],
        "protocol/protocol_clause"
    );
}

#[test]
#[trace("FSpec-FR-813-AC-24")]
fn a_present_role_binding_with_the_wrong_type_refuses_wrong_role_mapping() {
    let tail = format!(
        "{}protocol Hand using P over (k: Counter::Counter) on origin {{\n\
             role a on Counter::Counter;\n\
             requires temporal T;\n\
             run sequence Main {{ check Ok using v {{ true }}; }}\n\
             finish Done as (d: Counter::Counter) {{ true }};\n\
         }}\n",
        clause_tail("always holds(c.value <= 5)")
    );
    let (exit, value) = export_unit_with_protocol_role_type(
        &tail,
        Target::Tla,
        None,
        "filament.temporal.infinite-trace/v1",
        true,
        true,
        true,
        "example/counter-third",
        "c",
        json!({}),
        None,
        None,
        "object-binding",
    );
    assert_eq!(exit, 20, "{value}");
    assert_eq!(value["code"], "invalid_runtime_input", "{value}");
    assert_eq!(value["cause"], "wrong-role-mapping", "{value}");
    assert_eq!(
        value["details"]["path"], "/request/export/protocols/0/roles/a",
        "{value}"
    );
}

#[test]
#[trace("FSpec-FR-813-AC-24")]
fn protocol_roles_bind_to_their_own_model_in_a_two_model_package() {
    let declarations = "role a on Counter::Counter;\n\
                       role b on Spec::Counter;\n";
    let valid = export_two_model_protocol(
        declarations,
        json!({
            "a": {"type": "model-binding", "selection": "example/counter-third"},
            "b": {"type": "model-binding", "selection": "example/counter-spec"}
        }),
        "Counter::Counter",
    );
    assert_eq!(valid.0, 21, "{}", valid.1);
    assert_eq!(
        valid.1["export"]["refusal"]["construct"],
        "protocol/protocol_clause"
    );

    let swapped = export_two_model_protocol(
        declarations,
        json!({
            "a": {"type": "model-binding", "selection": "example/counter-spec"},
            "b": {"type": "model-binding", "selection": "example/counter-third"}
        }),
        "Counter::Counter",
    );
    assert_eq!(swapped.0, 20, "{}", swapped.1);
    assert_eq!(swapped.1["code"], "invalid_runtime_input", "{}", swapped.1);
    assert_eq!(swapped.1["cause"], "wrong-role-mapping", "{}", swapped.1);
    assert_eq!(
        swapped.1["details"]["path"], "/request/export/protocols/0/roles/a",
        "{}",
        swapped.1
    );
}

#[test]
#[trace("FR-425-AC-13", "FR-1015-AC-15")]
fn whole_model_roles_on_two_selected_models_are_auto_bound() {
    let (exit, value) = export_two_model_protocol(
        "role a on Counter;\nrole b on Spec;\n",
        json!({}),
        "Counter::Counter",
    );
    assert_eq!(exit, 21, "{value}");
    assert_eq!(
        value["export"]["refusal"]["construct"],
        "protocol/protocol_clause"
    );
}

#[test]
#[trace("FSpec-FR-813-AC-24")]
fn a_present_over_object_of_the_wrong_declared_type_refuses_role_mapping() {
    let tail = format!(
        "{}protocol Hand using P over (k: Counter::Counter) on origin {{\n\
             role a on Counter::Counter;\n\
             requires temporal T;\n\
             run sequence Main {{ check Ok using v {{ true }}; }}\n\
             finish Done as (d: Counter::Counter) {{ true }};\n\
         }}\n",
        clause_tail("always holds(c.value <= 5)")
    );
    let (exit, value) = export_unit_with_protocol_role_and_over_type(
        &tail,
        Target::Tla,
        None,
        "filament.temporal.infinite-trace/v1",
        true,
        true,
        true,
        "example/counter-third",
        "c",
        json!({}),
        None,
        None,
        "model-binding",
        Some("ix://example/counter-third/Level"),
    );
    assert_eq!(exit, 20, "{value}");
    assert_eq!(value["code"], "invalid_runtime_input", "{value}");
    assert_eq!(value["cause"], "wrong-role-mapping", "{value}");
    assert_eq!(
        value["details"]["path"], "/request/export/protocols/0/over",
        "{value}"
    );
}

#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-24")]
fn a_protocol_each_requirement_is_refused_at_its_activation_trigger() {
    let tail = format!(
        "{}protocol Hand using P over (k: Counter::Counter) on each (trg0: Boolean) {{\n\
             role a on Counter::Counter;\n\
             requires temporal T;\n\
             run sequence Main {{ check Ok using v {{ true }}; }}\n\
             finish Done as (d: Counter::Counter) {{ true }};\n\
         }}\n",
        clause_tail("always holds(c.value <= 5)")
    );
    for target in [Target::Tla, Target::Lean] {
        let refusal = refusal_of(&tail, target);
        assert_ne!(refusal["construct"], "temporal/temporal_clause");
        assert_ne!(refusal["construct"], "protocol/protocol_clause");
    }
}

/// FR-1015-AC-15: origin-required protocol subjects keep a typed refusal at
/// the protocol clause while the mapper does not lower protocol semantics.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-24")]
fn a_valid_symbolic_channel_capacity_reaches_the_protocol_typed_refusal() {
    let tail = format!(
        "{}protocol Hand using P over (k: Counter::Counter) on origin {{\n\
             role a on Counter::Counter;\n\
             role b on Counter::Counter;\n\
             channel ch from a to b carries Boolean ordering unordered delivery exactly-once-premise capacity symbolic(Cap) overflow block;\n\
             requires temporal T;\n\
             run sequence Main {{ check Ok using v {{ true }}; }}\n\
             finish Done as (d: Counter::Counter) {{ true }};\n\
         }}\n",
        clause_tail("always holds(c.value <= 5)")
    );
    for target in [Target::Tla, Target::Lean] {
        let (exit, value) = export_unit_with_protocol_options(
            &tail,
            target,
            None,
            "filament.temporal.infinite-trace/v1",
            true,
            true,
            true,
            "example/counter-third",
            "c",
            json!({"Cap": "1"}),
            None,
            None,
        );
        assert_eq!(exit, 21, "{value}");
        assert_eq!(
            value["export"]["refusal"]["code"], "unimplemented_capability",
            "{value}"
        );
        assert_eq!(
            value["export"]["refusal"]["cause"], "unimplemented-meaning",
            "{value}"
        );
        assert_eq!(
            value["export"]["refusal"]["construct"], "protocol/protocol_clause",
            "{value}"
        );
        let refusal = &value["export"]["refusal"];
        assert_eq!(refusal["construct"], "protocol/protocol_clause");
    }
}

#[test]
#[trace("FSpec-FR-813-AC-24")]
fn protocol_capacity_bindings_use_fr785_admission() {
    let tail = format!(
        "{}protocol Hand using P over (k: Counter::Counter) on origin {{\n\
             role a on Counter::Counter;\n\
             role b on Counter::Counter;\n\
             channel ch from a to b carries Boolean ordering unordered delivery exactly-once-premise capacity symbolic(Cap) overflow block;\n\
             requires temporal T;\n\
             run sequence Main {{ check Ok using v {{ true }}; }}\n\
             finish Done as (d: Counter::Counter) {{ true }};\n\
         }}\n",
        clause_tail("always holds(c.value <= 5)")
    );
    for (capacities, cause, name) in [
        (json!({"Unknown": "1"}), "unknown-member", "Unknown"),
        (json!({"Cap": "01"}), "invalid-value", "Cap"),
    ] {
        let (exit, value) = export_unit_with_protocol_options(
            &tail,
            Target::Tla,
            None,
            "filament.temporal.infinite-trace/v1",
            true,
            true,
            true,
            "example/counter-third",
            "c",
            capacities,
            None,
            None,
        );
        assert_eq!(exit, 20, "{value}");
        assert_eq!(value["code"], "invalid_runtime_input", "{value}");
        assert_eq!(value["cause"], cause, "{value}");
        assert_eq!(
            value["details"]["path"],
            format!("/request/export/protocols/0/capacities/{name}"),
            "{value}"
        );
    }
}

/// A nonliteral check or finish predicate remains within the same typed
/// protocol refusal boundary as every other protocol subject.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-17")]
fn a_protocol_subject_origin_with_a_nontrivial_check_or_finish_keeps_its_refusal() {
    let tail = format!(
        "{}protocol Hand using P over (k: Counter::Counter) on origin {{\n\
             role a on Counter::Counter;\n\
             requires temporal T;\n\
             run sequence Main {{ check Ok using v {{ true }}; }}\n\
             finish Done as (d: Counter::Counter) {{ true }};\n\
         }}\n",
        clause_tail("always holds(c.value <= 5)")
    );
    for unsupported in [
        tail.replace("check Ok using v { true }", "check Ok using v { false }"),
        tail.replace(
            "finish Done as (d: Counter::Counter) { true }",
            "finish Done as (d: Counter::Counter) { false }",
        ),
    ] {
        for target in [Target::Tla, Target::Lean] {
            let refusal = refusal_of(&unsupported, target);
            assert_eq!(refusal["construct"], "protocol/protocol_clause");
        }
    }
}

/// FSpec FR-812 and FR-813 (FND-013): a narrowing's range check is a
/// definedness condition of `ok_T`, never an atom the clause can negate.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-1")]
fn a_definedness_condition_is_in_ok_and_not_in_the_atom() {
    let body = "always (holds(c.value = 5) implies historically holds(c.value >= 0))";
    let tla = clause_text(body, Target::Tla);
    let ok = tla.lines().find(|l| l.starts_with("ok_T")).unwrap();
    assert!(ok.contains("(0 <= 5) /\\ (5 <= 9)"), "{tla}");
    assert!(
        !tla.lines()
            .filter(|l| !l.starts_with("ok_T"))
            .any(|l| l.contains("(0 <= 5)")),
        "{tla}"
    );
    let lean = clause_text(body, Target::Lean);
    let ok = lean.lines().find(|l| l.starts_with("def ok_T")).unwrap();
    assert!(ok.contains("(0 <= 5) /\\ (5 <= 9)"), "{lean}");
    assert!(
        !lean
            .lines()
            .filter(|l| !l.starts_with("def ok_T"))
            .any(|l| l.contains("(0 <= 5)")),
        "{lean}"
    );
}

/// FSpec FR-814-AC-8 (FND-019): a parameter spelled `self` is `p_self` and
/// the receiver stays `r_self`.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-814-AC-8")]
fn the_receiver_is_known_by_node_not_by_spelling() {
    let text = exported_text("always holds(c.value <= 5)", Target::Tla);
    assert!(text.contains("r_self"), "{text}");
    assert!(!text.contains("p_self"), "{text}");
}
