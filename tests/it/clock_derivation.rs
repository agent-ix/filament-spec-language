// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-640-AC-7 and AC-9: a clause over a model subject derives its clock
//! configuration under every profile, and library units read the request's
//! one `clocks` member.
use crate::command_harness::compile_file;
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const MODEL: &[u8] = include_bytes!("../../examples/config-version/model.semantic-ir.json");

fn source(file: &str, identity: &str) -> Value {
    json!({"file": file, "authority": "agent-ix", "identity": identity,
        "document": "Clocks", "formal_revision": 1})
}

fn write(directory: &Path, file: &str, text: &[u8]) {
    std::fs::write(directory.join(file), text).unwrap();
}

fn compile(directory: &Path, request: &Value) -> crate::command_harness::Output {
    let path: PathBuf = directory.join("compile.json");
    std::fs::write(&path, serde_json::to_vec(request).unwrap()).unwrap();
    compile_file(&path)
}

/// The value of the first `key` member anywhere in `value`.
fn find<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    match value {
        Value::Object(members) => members
            .get(key)
            .or_else(|| members.values().find_map(|member| find(member, key))),
        Value::Array(items) => items.iter().find_map(|item| find(item, key)),
        _ => None,
    }
}

#[trace("TC-2181", "FR-640-AC-7")]
#[test]
fn model_subject_clauses_derive_their_clock_configuration() {
    let directory = tempfile::tempdir().unwrap();
    write(directory.path(), "model.json", MODEL);
    let unit = "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         profile f = \"filament.temporal.fixed-sample.false-extension/v1\";\n\
         profile w = \"filament.temporal.timestamped-event.finite-window/v1\";\n\
         model Config = \"example/config-version\";\n\
         temporal Sampled using f over (c: Config::ConfigVersion) clock \"model-steps\" \
         on origin { eventually[0,3] holds(c.versionNumber >= 0) }\n\
         temporal Stamped using w over (c: Config::ConfigVersion) clock \"model-steps\" \
         on origin { eventually[0,3] holds(c.versionNumber >= 0) }\n";
    write(directory.path(), "unit.native", unit.as_bytes());
    let request = json!({"format": "native-compile/1", "request": {
        "models": [{"format": "semantic-ir/2.0.0",
            "source": source("model.json", "test:clocks-model")}],
        "program": {"source": source("unit.native", "test:clocks-unit")}}});
    let output = compile(directory.path(), &request);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let package: Value = serde_json::from_slice(&output.stdout).expect("package JSON");
    let rows = find(&package, "temporal_bindings")
        .and_then(Value::as_array)
        .expect("temporal binding rows");
    let clocks: Vec<&Value> = rows.iter().map(|row| &row["clock"]).collect();
    assert!(clocks.contains(&&json!({"kind": "fixed_sample",
        "epoch": {"kind": "integer", "decimal": "0"},
        "period": {"kind": "integer", "decimal": "1"}, "unit": "step"})));
    assert!(clocks.contains(&&json!({"kind": "timestamped_event", "timestamp_unit": "step"})));
}

#[trace("TC-2183", "FR-640-AC-9")]
#[test]
fn library_units_share_the_request_clocks() {
    let directory = tempfile::tempdir().unwrap();
    let library = "language \"ix:native\" edition \"1-draft\";\n\
         profile w = \"filament.temporal.timestamped-event.finite-window/v1\";\n\
         record Reading { level: Int[0, 9]; }\n\
         temporal Late using w over (r: Reading) clock \"plant-ms\" \
         on origin { eventually[0,5] holds(r.level > 0) }\n";
    let program = "language \"ix:native\" edition \"1-draft\";\n\
         profile w = \"filament.temporal.timestamped-event.finite-window/v1\";\n\
         import \"test/plant\" as P;\n\
         record Reading { level: Int[0, 9]; }\n\
         temporal Early using w over (r: Reading) clock \"plant-ms\" \
         on origin { eventually[0,2] holds(r.level > 0) }\n";
    write(directory.path(), "plant.native", library.as_bytes());
    write(directory.path(), "unit.native", program.as_bytes());
    let request = |clocks: Option<Value>| {
        let mut request = json!({"format": "native-compile/1", "request": {
            "models": [],
            "program": {"source": source("unit.native", "test:clocks-program")},
            "libraries": [{"identity": "test/plant",
                "source": source("plant.native", "test:clocks-plant")}]}});
        if let Some(clocks) = clocks {
            request["request"]["clocks"] = clocks;
        }
        request
    };
    let served = compile(
        directory.path(),
        &request(Some(
            json!({"plant-ms": {"kind": "timestamped_event", "timestamp_unit": "ms"}}),
        )),
    );
    assert_eq!(
        served.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&served.stderr)
    );
    let package: Value = serde_json::from_slice(&served.stdout).expect("package JSON");
    let rows = find(&package, "temporal_bindings")
        .and_then(Value::as_array)
        .expect("temporal binding rows");
    assert_eq!(
        rows.iter()
            .map(|row| (&row["clock_name"], &row["clock"]))
            .collect::<Vec<_>>(),
        [(
            &json!("plant-ms"),
            &json!({"kind": "timestamped_event", "timestamp_unit": "ms"})
        )]
    );
    let missing = compile(directory.path(), &request(None));
    assert_eq!(missing.status.code(), Some(20));
    let failure: Value = serde_json::from_slice(&missing.stderr).expect("a refusal");
    assert_eq!(
        find(&failure, "cause"),
        Some(&json!("missing-clock-selection"))
    );
    assert_eq!(find(&failure, "clock"), Some(&json!("plant-ms")));
    assert_eq!(find(&failure, "declaration"), Some(&json!("Late")));
}

#[trace("TC-2212", "FR-640-AC-10", "FSpec-TC-1661", "FSpec-FR-322-AC-452")]
#[test]
#[ignore = "QSL-2274: the program package does not record its libraries' clock selections yet"]
fn the_program_package_records_its_library_clock_selections() {
    let directory = tempfile::tempdir().unwrap();
    let library = "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         profile w = \"filament.temporal.timestamped-event.finite-window/v1\";\n\
         record Reading { level: Int[0, 9]; }\n\
         function flag using v(): Boolean pure { true }\n\
         temporal Late using w over (r: Reading) clock \"lib-ms\" \
         on origin { eventually[0,5] holds(r.level > 0) }\n";
    let plain = "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         function flag using v(): Boolean pure { true }\n";
    let program = "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         import \"test/plant\" as P;\n\
         function entry using v(): Boolean pure { P::flag() }\n";
    write(directory.path(), "plant.native", library.as_bytes());
    write(directory.path(), "plain.native", plain.as_bytes());
    write(directory.path(), "unit.native", program.as_bytes());
    let request = |file: &str, clocks: Option<Value>| {
        let mut request = json!({"format": "native-compile/1", "request": {
            "models": [],
            "program": {"source": source("unit.native", "test:clocks-program")},
            "libraries": [{"identity": "test/plant",
                "source": source(file, "test:clocks-plant")}]}});
        if let Some(clocks) = clocks {
            request["request"]["clocks"] = clocks;
        }
        request
    };
    let compiled = |unit: &str| {
        let output = compile(
            directory.path(),
            &request(
                "plant.native",
                Some(json!({"lib-ms": {"kind": "timestamped_event", "timestamp_unit": unit}})),
            ),
        );
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        output.stdout
    };
    let ms = compiled("ms");
    let package: Value = serde_json::from_slice(&ms).expect("package JSON");
    assert_eq!(
        package["lock"]["dependency_clock_selections"],
        json!([{"clock_name": "lib-ms",
            "clock": {"kind": "timestamped_event", "timestamp_unit": "ms"}}])
    );
    assert_ne!(ms, compiled("s"));

    let output = compile(directory.path(), &request("plain.native", None));
    assert_eq!(output.status.code(), Some(0));
    let package: Value = serde_json::from_slice(&output.stdout).expect("package JSON");
    assert!(package["lock"].get("dependency_clock_selections").is_none());
}

/// FR-640-AC-9: a member only a library unit's clause reads is used: a
/// program with no clock-bearing clause of its own compiles with it, and a
/// member no unit reads still refuses `unused-clock-selection`.
#[trace("TC-2183", "FR-640-AC-9")]
#[test]
fn a_member_only_a_library_clause_reads_is_not_unused() {
    let directory = tempfile::tempdir().unwrap();
    let library = "language \"ix:native\" edition \"1-draft\";\n\
         profile w = \"filament.temporal.timestamped-event.finite-window/v1\";\n\
         record Reading { level: Int[0, 9]; }\n\
         temporal Late using w over (r: Reading) clock \"plant-ms\" \
         on origin { eventually[0,5] holds(r.level > 0) }\n";
    let program = "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         import \"test/plant\" as P;\n\
         function f using v(x: Int[0, 9]): Boolean pure { x < 5 }\n";
    write(directory.path(), "plant.native", library.as_bytes());
    write(directory.path(), "unit.native", program.as_bytes());
    let clock = json!({"kind": "timestamped_event", "timestamp_unit": "ms"});
    let request = json!({"format": "native-compile/1", "request": {
        "models": [],
        "program": {"source": source("unit.native", "test:clocks-program")},
        "libraries": [{"identity": "test/plant",
            "source": source("plant.native", "test:clocks-plant")}],
        "clocks": {"plant-ms": clock.clone()}}});
    let output = compile(directory.path(), &request);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut spare = request.clone();
    spare["request"]["clocks"]["spare"] = clock;
    let unused = compile(directory.path(), &spare);
    assert_eq!(unused.status.code(), Some(20));
    let failure: Value = serde_json::from_slice(&unused.stderr).expect("a refusal");
    assert_eq!(
        find(&failure, "cause"),
        Some(&json!("unused-clock-selection"))
    );
    assert_eq!(find(&failure, "clock"), Some(&json!("spare")));
}

/// The ConfigVersion model document republished under `identity`.
fn model_as(identity: &str) -> Vec<u8> {
    String::from_utf8(MODEL.to_vec())
        .unwrap()
        .replace("example/config-version", identity)
        .into_bytes()
}

/// `command::compile` of `unit` against one model document per
/// `(alias identity, file)`.
fn compile_over_models(
    directory: &Path,
    unit: &str,
    models: &[&str],
) -> crate::command_harness::Output {
    let mut entries = Vec::new();
    for (index, identity) in models.iter().enumerate() {
        let file = format!("model-{index}.json");
        write(directory, &file, &model_as(identity));
        entries.push(json!({"format": "semantic-ir/2.0.0",
            "source": source(&file, &format!("test:clocks-model-{index}"))}));
    }
    write(directory, "unit.native", unit.as_bytes());
    compile(
        directory,
        &json!({"format": "native-compile/1", "request": {
            "models": entries,
            "program": {"source": source("unit.native", "test:clocks-unit")}}}),
    )
}

/// The declaration identity text of `example/<name>`'s `ConfigVersion`, the
/// sequence authority a trace of that model's steps derives (FR-640-AC-3).
fn config_version_authority(package: &str) -> String {
    format!("{{\"node\":\"ix://{package}/ConfigVersion\",\"package\":\"{package}\"}}")
}

/// FR-640 "Behavior": a hyper clause over one model's behaviours derives its
/// event-position configuration from the model, per quantified trace, as a
/// single-trace clause over that model's type does: the sequence authority
/// is the type's declaration identity text, no `clocks` member serves it,
/// and each trace has its own row with an equal configuration.
#[trace("TC-2243", "FR-640-AC-12", "FSpec-TC-1680", "FSpec-FR-322-AC-453")]
#[test]
fn a_hyper_clause_over_one_model_derives_the_single_trace_authority() {
    let directory = tempfile::tempdir().unwrap();
    let unit = "language \"ix:native\" edition \"1-draft\";\n\
         profile e = \"filament.temporal.event-position.false-extension/v1\";\n\
         model Config = \"example/config-version\";\n\
         hyper Same using e over behaviours (c: Config::ConfigVersion) \
         forall trace a of Config forall trace b of Config \
         { always[0,3] holds(c.versionNumber @ a = c.versionNumber @ b) }\n";
    let output = compile_over_models(directory.path(), unit, &["example/config-version"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let package: Value = serde_json::from_slice(&output.stdout).expect("package JSON");
    let rows = find(&package, "temporal_bindings")
        .and_then(Value::as_array)
        .expect("temporal binding rows");
    let [first, second] = rows.as_slice() else {
        panic!("one binding row per quantified trace: {rows:?}");
    };
    for (row, trace) in [(first, "a"), (second, "b")] {
        assert_eq!(row["trace"], trace, "{row}");
        assert_eq!(row["clock_name"], "model-steps");
        assert_eq!(
            row["clock"],
            json!({"kind": "event_position",
                "sequence_authority": config_version_authority("example/config-version")})
        );
    }
}

/// FR-640-AC-12: a hyper clause whose two quantified traces are of two
/// models derives two event-position configurations, one row per trace,
/// each with its own model's sequence authority. Under fixed-sample both
/// traces derive the same configuration, and the clause compiles too.
#[trace("TC-2243", "FR-640-AC-12", "FSpec-TC-1680", "FSpec-FR-322-AC-453")]
#[test]
#[ignore = "QSL-2277: a hyper clause over two models under event-position compiles with one clock row per trace, not implemented yet"]
fn a_hyper_clause_over_two_models_binds_one_clock_per_trace() {
    let directory = tempfile::tempdir().unwrap();
    let unit = |identity: &str| {
        format!(
            "language \"ix:native\" edition \"1-draft\";\n\
             profile t = \"{identity}\";\n\
             model S = \"example/device-saving\";\n\
             model N = \"example/device-normal\";\n\
             hyper Saves using t over behaviours (d: S::ConfigVersion, e: N::ConfigVersion) \
             forall trace s of S forall trace n of N \
             {{ always[0,3] holds(d.versionNumber @ s <= e.versionNumber @ n) }}\n"
        )
    };
    let models = ["example/device-saving", "example/device-normal"];
    let compiled = compile_over_models(
        directory.path(),
        &unit("filament.temporal.event-position.false-extension/v1"),
        &models,
    );
    assert_eq!(
        compiled.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let package: Value = serde_json::from_slice(&compiled.stdout).expect("package JSON");
    let rows = find(&package, "temporal_bindings")
        .and_then(Value::as_array)
        .expect("temporal binding rows");
    let [saving, normal] = rows.as_slice() else {
        panic!("one binding row per quantified trace: {rows:?}");
    };
    for (row, trace, model) in [
        (saving, "s", "example/device-saving"),
        (normal, "n", "example/device-normal"),
    ] {
        assert_eq!(row["trace"], trace, "{row}");
        assert_eq!(row["clock_name"], "model-steps");
        assert_eq!(
            row["clock"],
            json!({"kind": "event_position",
                "sequence_authority": config_version_authority(model)})
        );
    }
    let sampled = compile_over_models(
        directory.path(),
        &unit("filament.temporal.fixed-sample.false-extension/v1"),
        &models,
    );
    assert_eq!(
        sampled.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&sampled.stderr)
    );
}

/// FR-326-AC-4 (TC-836): a clause whose `using` alias selects the value
/// profile refuses `unknown_profile`/`unsupported-selection`, and the message
/// names the alias and the identity it selects.
#[trace("TC-836", "FR-326-AC-4")]
#[test]
fn a_clause_under_a_value_profile_alias_names_the_alias_and_its_selection() {
    let directory = tempfile::tempdir().unwrap();
    let unit = "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         record Reading { level: Int[0, 9]; }\n\
         temporal C using v over (r: Reading) clock \"events\" \
         on origin { eventually[0,1] holds(r.level = 3) }\n";
    let output = compile_over_models(directory.path(), unit, &[]);
    assert_eq!(output.status.code(), Some(20));
    let failure: Value = serde_json::from_slice(&output.stderr).expect("a refusal");
    assert_eq!(failure["code"], "unknown_profile", "{failure}");
    assert_eq!(failure["cause"], "unsupported-selection", "{failure}");
    let message = failure["message"].as_str().expect("a message");
    assert!(
        message.contains("`v`") && message.contains("`filament.value.complete/v1`"),
        "{message}"
    );
}

/// `package` with each operand slot replaced by the node index it names.
fn resolved(mut package: Value) -> Value {
    fsl_semantics::check::wire_id::resolve_operand_slots(&mut package)
        .expect("every operand slot names a row entry");
    package
}

/// The `claim`/`hyperproperty` nodes of `package`, each with the NodeId of
/// the body wrapper its `body` binding references (absent for a finite
/// domain, whose body is a Boolean expression).
fn hyper_nodes(package: &Value) -> Vec<(&Value, Option<&Value>)> {
    package["semantic_graph"]["nodes"]
        .as_array()
        .expect("graph nodes")
        .iter()
        .filter(|node| node["semantic_form"] == "hyperproperty")
        .map(|node| {
            let body = node["body"]["members"]
                .as_array()
                .and_then(|members| members.iter().find(|member| member["name"] == "body"))
                .map(|member| &member["value"]["target"]);
            let wrapper = body.filter(|target| {
                package["semantic_graph"]["nodes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|node| {
                        &node["node_id"] == *target && node["semantic_form"] == "temporal_clause"
                    })
            });
            (&node["node_id"], wrapper)
        })
        .collect()
}

/// The `trace` members of the binding rows keyed by `declaration`, in row order.
fn traces_at<'a>(rows: &'a [Value], declaration: &Value) -> Vec<&'a str> {
    rows.iter()
        .filter(|row| &row["declaration"] == declaration)
        .map(|row| row["trace"].as_str().expect("a trace member"))
        .collect()
}

/// FR-488-AC-25: a hyper clause's per-trace binding rows are keyed by its
/// `hyperproperty` node and its `claim` occurrence, never by its body
/// wrapper, which has no binding row.
#[trace("TC-1327", "FR-488-AC-25", "FSpec-TC-1680", "FSpec-FR-322-AC-453")]
#[test]
fn hyper_binding_rows_are_keyed_by_the_hyperproperty_node() {
    let directory = tempfile::tempdir().unwrap();
    let unit = "language \"ix:native\" edition \"1-draft\";\n\
         profile e = \"filament.temporal.event-position.false-extension/v1\";\n\
         model Config = \"example/config-version\";\n\
         hyper Same using e over behaviours (c: Config::ConfigVersion) \
         forall trace a of Config forall trace b of Config \
         { always[0,3] holds(c.versionNumber @ a = c.versionNumber @ b) }\n";
    let output = compile_over_models(directory.path(), unit, &["example/config-version"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let package: Value = serde_json::from_slice(&output.stdout).expect("package JSON");
    let package = resolved(package);
    let rows = find(&package, "temporal_bindings")
        .and_then(Value::as_array)
        .expect("rows");
    let [(node, Some(wrapper))] = hyper_nodes(&package)[..] else {
        panic!("one hyperproperty node with a wrapper");
    };
    assert_eq!(traces_at(rows, node), ["a", "b"]);
    assert!(
        traces_at(rows, wrapper).is_empty(),
        "no row names the wrapper: {rows:?}"
    );
}

/// The compile of two hyper clauses with equal bodies and reversed prefixes.
fn reversed_prefix_package() -> Value {
    let directory = tempfile::tempdir().unwrap();
    let unit = "language \"ix:native\" edition \"1-draft\";\n\
         profile e = \"filament.temporal.event-position.false-extension/v1\";\n\
         model Config = \"example/config-version\";\n\
         hyper Fwd using e over behaviours (c: Config::ConfigVersion) \
         forall trace a of Config forall trace b of Config \
         { always[0,3] holds(c.versionNumber @ a = c.versionNumber @ b) }\n\
         hyper Rev using e over behaviours (c: Config::ConfigVersion) \
         forall trace b of Config forall trace a of Config \
         { always[0,3] holds(c.versionNumber @ a = c.versionNumber @ b) }\n";
    let output = compile_over_models(directory.path(), unit, &["example/config-version"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    resolved(serde_json::from_slice(&output.stdout).expect("package JSON"))
}

/// FR-488-AC-26: two hyper clauses with equal bodies and reversed prefixes
/// each emit their rows on their own `hyperproperty` node, in their own
/// quantifier order, and none on a body wrapper.
#[trace("TC-3000", "FR-488-AC-26", "FSpec-TC-2027", "FSpec-FR-322-AC-463")]
#[test]
fn reversed_prefix_hyper_clauses_emit_their_own_trace_rows() {
    let package = reversed_prefix_package();
    let rows = find(&package, "temporal_bindings")
        .and_then(Value::as_array)
        .expect("rows");
    let nodes = hyper_nodes(&package);
    let [(first, Some(wrapper)), (second, Some(other))] = nodes[..] else {
        panic!("two hyperproperty nodes with wrappers: {nodes:?}");
    };
    assert!(traces_at(rows, wrapper).is_empty());
    assert!(traces_at(rows, other).is_empty());
    let mut orders = [traces_at(rows, first), traces_at(rows, second)];
    orders.sort();
    assert_eq!(orders, [vec!["a", "b"], vec!["b", "a"]]);
}

/// FR-488-AC-26: the two clauses' `body` references one wrapper.
#[trace("TC-3000", "FR-488-AC-26", "FSpec-TC-2027", "FSpec-FR-322-AC-463")]
#[test]
#[ignore = "QSL-2400: the slots of reversed prefixes sit at different levels, so the two clauses get two body wrappers, not one"]
fn reversed_prefix_hyper_clauses_share_one_body_wrapper() {
    let package = reversed_prefix_package();
    let nodes = hyper_nodes(&package);
    let [(_, Some(wrapper)), (_, Some(shared))] = nodes[..] else {
        panic!("two hyperproperty nodes with wrappers: {nodes:?}");
    };
    assert_eq!(wrapper, shared, "the two clauses share one wrapper");
}

/// FR-898-AC-7: a finite-domain hyper clause emits one row per trace at its
/// `hyperproperty` node, each carrying the domain binder `t` as its clock
/// role and the configuration the request binds to it, under the lock's
/// `temporal_profile` selection.
#[trace("TC-3001", "FR-898-AC-7", "FSpec-TC-2028", "FSpec-FR-322-AC-464")]
#[test]
fn a_finite_domain_hyper_clause_emits_one_row_per_trace_on_its_domain_binder() {
    let directory = tempfile::tempdir().unwrap();
    let unit = "language \"ix:native\" edition \"1-draft\";\n\
         profile e = \"filament.temporal.event-position.false-extension/v1\";\n\
         record Obs { inp: Int[0, 3]; out: Int[0, 3]; }\n\
         hyper Det using e over t in Obs bounded 3 forall trace a forall trace b \
         { (a.inp = b.inp) -> (a.out = b.out) }\n";
    write(directory.path(), "unit.native", unit.as_bytes());
    let clock = json!({"kind": "event_position", "sequence_authority": "obs"});
    let request = |clocks: Option<Value>| {
        let mut request = json!({"format": "native-compile/1", "request": {
            "models": [],
            "program": {"source": source("unit.native", "test:clocks-finite")}}});
        if let Some(clocks) = clocks {
            request["request"]["clocks"] = clocks;
        }
        request
    };
    let output = compile(
        directory.path(),
        &request(Some(json!({"t": clock.clone()}))),
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let package: Value = serde_json::from_slice(&output.stdout).expect("package JSON");
    let rows = find(&package, "temporal_bindings")
        .and_then(Value::as_array)
        .expect("rows");
    let [(node, None)] = hyper_nodes(&package)[..] else {
        panic!("one finite-domain hyperproperty node");
    };
    assert_eq!(traces_at(rows, node), ["a", "b"]);
    for row in rows {
        assert_eq!(row["clock_name"], "t", "{row}");
        assert_eq!(row["clock"], clock, "{row}");
    }
    for refused in [
        None,
        Some(json!({"t": clock.clone(), "spare": clock.clone()})),
        Some(json!({"t": {"kind": "fixed_sample",
            "epoch": {"kind": "integer", "decimal": "0"},
            "period": {"kind": "integer", "decimal": "1"}, "unit": "ms"}})),
    ] {
        let output = compile(directory.path(), &request(refused.clone()));
        assert_eq!(output.status.code(), Some(20), "{refused:?}");
        assert!(output.stdout.is_empty(), "no package for {refused:?}");
    }
}

/// FR-488-AC-27 and FSpec FR-402-AC-10: two quantifier Groups with one
/// `variable` text refuse at FR-402's Shape step with
/// `ambiguous_declaration`/`ambiguous-name` at the later Group's `variable`
/// binding, so each Group keys exactly one clock row.
#[trace("TC-3007", "FR-488-AC-27")]
#[trace("FSpec-TC-346", "FSpec-FR-402-AC-10")]
#[test]
#[ignore = "QSL-2400: a repeated quantifier variable refused at the hyperproperty Shape step not implemented yet"]
fn a_repeated_quantifier_variable_is_refused_at_the_hyperproperty_shape() {
    use filament_contract_model::{
        CheckedPackageEvidence, CheckedPackageReadLimits, CheckedPackageRefusalCause,
        CheckedPackageRefusalCode, CheckedPackageV2, CheckedPackageV2ReadResult, JsonPointer,
    };

    let directory = tempfile::tempdir().unwrap();
    let unit = "language \"ix:native\" edition \"1-draft\";\n\
         profile i = \"filament.temporal.infinite-trace/v1\";\n\
         model Config = \"example/config-version\";\n\
         hyper Same using i over behaviours (c: Config::ConfigVersion) \
         forall trace a of Config forall trace b of Config \
         { always holds(c.versionNumber @ a = c.versionNumber @ b) }\n";
    let output = compile_over_models(directory.path(), unit, &["example/config-version"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let model = model_as("example/config-version");
    let read = |bytes: &[u8]| {
        let wire: Value = serde_json::from_slice(bytes).expect("package JSON");
        let mut evidence = CheckedPackageEvidence::new();
        for feature in wire["lock"]["required_features"].as_array().unwrap() {
            evidence.support_feature(feature.as_str().unwrap());
        }
        for (identity, document) in
            fsl_semantics::model::intake::package_input([model.as_slice()]).unwrap()
        {
            evidence
                .insert_domain_package_document(identity.as_str(), document)
                .expect("a fresh evidence key");
        }
        CheckedPackageV2::read(bytes, CheckedPackageReadLimits::bounded(), &evidence)
    };
    assert!(matches!(
        read(&output.stdout),
        CheckedPackageV2ReadResult::Admitted(_)
    ));

    let mut package: Value = serde_json::from_slice(&output.stdout).expect("package JSON");
    let nodes = package["semantic_graph"]["nodes"]
        .as_array_mut()
        .expect("graph nodes");
    let n = nodes
        .iter()
        .position(|node| node["semantic_form"] == "hyperproperty")
        .expect("one hyperproperty node");
    let variable = &mut nodes[n]["body"]["members"][3]["value"]["members"][1]["members"][0];
    assert_eq!(variable["name"], "variable", "{variable}");
    variable["value"]["value"] = json!("a");
    let edited = serde_json::to_vec(&package).unwrap();

    let CheckedPackageV2ReadResult::Refused(refusal) = read(&edited) else {
        panic!("a repeated quantifier variable is refused");
    };
    assert_eq!(
        refusal.code,
        CheckedPackageRefusalCode::AmbiguousDeclaration
    );
    assert_eq!(
        refusal.cause,
        Some(CheckedPackageRefusalCause::AmbiguousName)
    );
    assert_eq!(
        refusal.loci[0].path,
        JsonPointer::parse(&format!(
            "/semantic_graph/nodes/{n}/body/members/3/value/members/1/members/0"
        ))
    );
}

/// `command::compile` of `unit` over the config-version model with a
/// `handoff` member, the handoff directory under `directory`.
fn compile_with_handoff(directory: &Path, unit: &str) -> crate::command_harness::Output {
    write(
        directory,
        "model-0.json",
        &model_as("example/config-version"),
    );
    write(directory, "unit.native", unit.as_bytes());
    compile(
        directory,
        &json!({"format": "native-compile/1", "request": {
            "models": [{"format": "semantic-ir/2.0.0",
                "source": source("model-0.json", "test:clocks-model-0")}],
            "program": {"source": source("unit.native", "test:clocks-unit")},
            "handoff": {"directory": "handoff", "package_name": "clocks",
                "native_join": {"work_units": 1_000_000, "storage": 1_000_000}}}}),
    )
}

/// FR-517-AC-25 (TC-3128): a `native-compile/1` request carrying the
/// `handoff` member over a unit with a temporal clause and a hyper clause
/// exits 0 and writes a directory that compiles back through its sidecar.
#[trace("TC-3128", "FR-517-AC-25")]
#[test]
fn a_compile_handoff_over_a_hyper_clause_exits_zero() {
    let directory = tempfile::tempdir().unwrap();
    let unit = "language \"ix:native\" edition \"1-draft\";\n\
         profile i = \"filament.temporal.infinite-trace/v1\";\n\
         model Config = \"example/config-version\";\n\
         temporal Bounded using i over (view: Config::ConfigVersion) clock \"model-steps\" \
         on origin { always holds(view.versionNumber = view.versionNumber) }\n\
         hyper Equal using i over behaviours (c: Config::ConfigVersion) \
         forall trace a of Config forall trace b of Config \
         { always holds(c.versionNumber @ a = c.versionNumber @ b) }\n";
    let output = compile_with_handoff(directory.path(), unit);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    fsl_command::checked_handoff::compile(&directory.path().join("handoff"))
        .expect("the handoff compiles back");
}

/// FR-517-AC-26 (TC-3128): the clock the compile derives for a clocked hyper
/// clause survives the handoff: the sidecar expects one row per quantifier
/// and the directory compiles back to the same bytes.
#[trace("TC-3128", "FR-517-AC-26")]
#[test]
fn a_compile_handoff_over_a_clocked_hyper_clause_keeps_its_per_trace_clocks() {
    let directory = tempfile::tempdir().unwrap();
    let unit = "language \"ix:native\" edition \"1-draft\";\n\
         profile e = \"filament.temporal.event-position.false-extension/v1\";\n\
         model Config = \"example/config-version\";\n\
         hyper Same using e over behaviours (c: Config::ConfigVersion) \
         forall trace a of Config forall trace b of Config \
         { always[0,3] holds(c.versionNumber @ a = c.versionNumber @ b) }\n";
    let output = compile_with_handoff(directory.path(), unit);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let handoff = directory.path().join("handoff");
    let sidecar: Value =
        serde_json::from_slice(&std::fs::read(handoff.join("selection.json")).unwrap()).unwrap();
    let traces = sidecar["hypers"][0]["traces"].as_array().expect("traces");
    assert_eq!(traces.len(), 2, "{sidecar}");
    for row in traces {
        assert_eq!(
            row["binding"]["clock"]["sequence_authority"],
            config_version_authority("example/config-version"),
            "{row}"
        );
    }
    fsl_command::checked_handoff::compile(&handoff).expect("the handoff compiles back");
}
