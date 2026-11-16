// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-640: the `clocks` member of native-compile/1 and native-run/1 requests
//! through the library `command` over request files.
//!
//! Every unit here declares its clauses over a record trace subject. One
//! unit mixes the event-position, fixed-sample and timestamped-event
//! profiles under three aliases (FR-326-AC-7); the others select one each.

use crate::command_harness::{compile_file, run_file, Output};
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const EVENTS: &str = "filament.temporal.event-position.false-extension/v1";
const SAMPLED: &str = "filament.temporal.fixed-sample.false-extension/v1";
const STAMPED: &str = "filament.temporal.timestamped-event.finite-window/v1";

/// A unit under the temporal profile `identity` whose `clauses` are over
/// `r: Reading` and use the alias `t`, with the value function `seven`.
fn unit(identity: &str, clauses: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         profile t = \"{identity}\";\n\
         record Reading {{ level: Int[0, 9]; }}\n\
         {clauses}\
         function seven using v(): Int[0, 9] pure {{ 7 }}\n"
    )
}

/// Two event-position clauses on the clock `events` and one on `audit`.
fn event_unit() -> String {
    unit(
        EVENTS,
        "temporal E using t over (r: Reading) clock \"events\" on origin { eventually[0,2] holds(r.level = 2) }\n\
         temporal Again using t over (r: Reading) clock \"events\" on origin { always[0,1] holds(r.level < 9) }\n\
         temporal Audit using t over (r: Reading) clock \"audit\" on origin { eventually[0,1] holds(r.level = 1) }\n",
    )
}

fn sample_unit() -> String {
    unit(
        SAMPLED,
        "temporal S using t over (r: Reading) clock \"sample\" on origin { eventually[0,3] holds(r.level = 2) }\n",
    )
}

fn stamped_unit() -> String {
    unit(
        STAMPED,
        "temporal T using t over (r: Reading) clock \"plant-ms\" on origin { eventually[0,5] holds(r.level > 0) }\n",
    )
}

/// One unit whose clauses use three temporal profiles under the aliases
/// `e`, `f` and `w`, one clock each (FR-640-AC-1).
fn mixed_unit() -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         profile e = \"{EVENTS}\";\n\
         profile f = \"{SAMPLED}\";\n\
         profile w = \"{STAMPED}\";\n\
         record Reading {{ level: Int[0, 9]; }}\n\
         temporal E using e over (r: Reading) clock \"events\" on origin {{ eventually[0,2] holds(r.level = 2) }}\n\
         temporal S using f over (r: Reading) clock \"sample\" on origin {{ eventually[0,3] holds(r.level = 2) }}\n\
         temporal T using w over (r: Reading) clock \"plant-ms\" on origin {{ eventually[0,5] holds(r.level > 0) }}\n\
         function seven using v(): Int[0, 9] pure {{ 7 }}\n"
    )
}

/// The `clocks` member configuring [`mixed_unit`]'s three clocks.
fn mixed_clocks() -> Value {
    json!({
        "events": {"kind": "event_position", "sequence_authority": "events"},
        "sample": sample_clocks()["sample"].clone(),
        "plant-ms": stamped_clocks()["plant-ms"].clone(),
    })
}

fn event_clocks() -> Value {
    json!({
        "events": {"kind": "event_position", "sequence_authority": "events"},
        "audit": {"kind": "event_position", "sequence_authority": "audit-log"},
    })
}

fn sample_clocks() -> Value {
    json!({"sample": {"kind": "fixed_sample",
        "epoch": {"kind": "integer", "decimal": "0"},
        "period": {"kind": "integer", "decimal": "1"}, "unit": "s"}})
}

fn stamped_clocks() -> Value {
    json!({"plant-ms": {"kind": "timestamped_event", "timestamp_unit": "ms"}})
}

fn program() -> Value {
    json!({"source": {"file": "unit.native", "authority": "agent-ix",
        "identity": "test:clock-selection", "document": "ClockSelection", "formal_revision": 1}})
}

fn compile_request(clocks: Value) -> Value {
    json!({"format": "native-compile/1", "request": {
        "models": [], "program": program(), "clocks": clocks}})
}

/// Writes `unit` and `request` (as `name`) into `directory`.
fn write(directory: &Path, unit: &str, name: &str, request: &Value) -> PathBuf {
    std::fs::write(directory.join("unit.native"), unit).unwrap();
    let path = directory.join(name);
    std::fs::write(&path, serde_json::to_vec(request).unwrap()).unwrap();
    path
}

/// `command::compile` of `unit` under `clocks`.
fn compile(directory: &Path, unit: &str, name: &str, clocks: Value) -> Output {
    compile_file(&write(directory, unit, name, &compile_request(clocks)))
}

/// The `temporal_bindings` rows of a compiled package.
fn rows(output: &Output) -> Vec<Value> {
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let package: Value = serde_json::from_slice(&output.stdout).expect("package JSON");
    find(&package, "temporal_bindings")
        .and_then(Value::as_array)
        .cloned()
        .expect("temporal binding rows")
}

/// The first value under `key` anywhere in `value`, depth first.
fn find<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    match value {
        Value::Object(members) => members
            .get(key)
            .or_else(|| members.values().find_map(|member| find(member, key))),
        Value::Array(items) => items.iter().find_map(|item| find(item, key)),
        _ => None,
    }
}

fn refusal(output: &Output) -> Value {
    assert!(output.stdout.is_empty(), "expected a refusal envelope");
    serde_json::from_slice(&output.stderr).expect("a refusal envelope")
}

/// FR-640-AC-1 (TC-1660): `clocks` configures the event-position,
/// fixed-sample and timestamped-event clocks of record-subject clauses by
/// clock name, in one unit that mixes the three profiles and in units of one
/// profile each. The package holds one `temporal_bindings` row per clause,
/// carrying its clock name and the member's configuration; the two clauses
/// on `events` get equal configurations from the one member.
#[test]
#[trace("TC-1660", "FR-640-AC-1", "FSpec-TC-1160", "FSpec-FR-540-AC-1")]
fn clocks_member_binds_each_trace_subject_clock_by_name() {
    let directory = tempfile::tempdir().unwrap();
    for (unit, clocks, expected) in [
        (
            mixed_unit(),
            mixed_clocks(),
            vec![
                ("events", "event_position"),
                ("sample", "fixed_sample"),
                ("plant-ms", "timestamped_event"),
            ],
        ),
        (
            event_unit(),
            event_clocks(),
            vec![
                ("events", "event_position"),
                ("events", "event_position"),
                ("audit", "event_position"),
            ],
        ),
        (
            sample_unit(),
            sample_clocks(),
            vec![("sample", "fixed_sample")],
        ),
        (
            stamped_unit(),
            stamped_clocks(),
            vec![("plant-ms", "timestamped_event")],
        ),
    ] {
        let rows = rows(&compile(
            directory.path(),
            &unit,
            "compile.json",
            clocks.clone(),
        ));
        assert_eq!(rows.len(), expected.len(), "{rows:?}");
        for (clock, kind) in expected {
            let bound: Vec<_> = rows
                .iter()
                .filter(|row| row["clock_name"] == clock)
                .collect();
            assert!(!bound.is_empty(), "a row for {clock}: {rows:?}");
            for row in bound {
                assert_eq!(row["clock"]["kind"], kind, "{row}");
                assert_eq!(row["clock"], clocks[clock], "{row}");
            }
        }
    }
}

/// FR-640-AC-8 (TC-2182): the `clocks` object rebuilt from a package's
/// `temporal_bindings` rows, each row's `clock_name` naming a member whose
/// value is the row's `clock`, compiles the same program to byte-identical
/// package bytes. It is the selection list the replay recompile builds.
#[test]
#[trace("TC-2182", "FR-640-AC-8")]
fn clocks_rebuilt_from_the_rows_compile_the_same_bytes() {
    let directory = tempfile::tempdir().unwrap();
    let first = compile(
        directory.path(),
        &mixed_unit(),
        "first.json",
        mixed_clocks(),
    );
    let mut rebuilt = serde_json::Map::new();
    for row in rows(&first) {
        let name = row["clock_name"].as_str().expect("a clock name").to_owned();
        rebuilt.insert(name, row["clock"].clone());
    }
    assert_eq!(Value::Object(rebuilt.clone()), mixed_clocks());
    let second = compile(
        directory.path(),
        &mixed_unit(),
        "second.json",
        Value::Object(rebuilt),
    );
    assert_eq!(second.status.code(), Some(0));
    assert_eq!(first.stdout, second.stdout);
}

/// FR-640-AC-5 (TC-1660): a changed `events` sequence authority changes the
/// package bytes; reordering the `clocks` members does not.
#[test]
#[trace("TC-1660", "FR-640-AC-5", "FSpec-TC-1160", "FSpec-FR-540-AC-3")]
fn clock_configuration_enters_package_identity_and_member_order_does_not() {
    let directory = tempfile::tempdir().unwrap();
    let base = compile(directory.path(), &event_unit(), "a.json", event_clocks());
    let mut other = event_clocks();
    other["events"]["sequence_authority"] = json!("elsewhere");
    let changed = compile(directory.path(), &event_unit(), "b.json", other);
    // The JSON text itself lists the members in the other order: a parsed
    // `Value` object would sort them again.
    let members = |name: &str| format!("{name:?}:{}", event_clocks()[name]);
    let text = serde_json::to_string(&compile_request(json!("CLOCKS")))
        .unwrap()
        .replace(
            "\"CLOCKS\"",
            &format!("{{{},{}}}", members("events"), members("audit")),
        );
    assert!(text.find("\"events\"") < text.find("\"audit\""), "{text}");
    std::fs::write(directory.path().join("unit.native"), event_unit()).unwrap();
    let reordered = directory.path().join("c.json");
    std::fs::write(&reordered, text).unwrap();
    let same = compile_file(&reordered);
    assert_eq!(base.status.code(), Some(0));
    assert_eq!(changed.status.code(), Some(0));
    assert_ne!(base.stdout, changed.stdout);
    assert_eq!(base.stdout, same.stdout);
}

/// FR-640-AC-3 (TC-1661): a missing, an unused and a mismatched selection
/// each refuse at stage `emit`, `invalid-request`, exit 20, naming the clock
/// and its cause in FR-640's message. The two faults one declaration owns
/// name that declaration and its profile and locate its `clock` argument.
#[test]
#[trace("TC-1661", "FR-640-AC-3", "FSpec-TC-1160", "FSpec-FR-540-AC-2")]
fn missing_unused_and_mismatched_selections_refuse_naming_the_clock() {
    let directory = tempfile::tempdir().unwrap();
    let mut unused = stamped_clocks();
    unused["spare"] = json!({"kind": "timestamped_event", "timestamp_unit": "ms"});
    let mut mismatched = event_clocks();
    mismatched["events"] = json!({"kind": "timestamped_event", "timestamp_unit": "ms"});
    for (index, (unit, selection, cause, clock, declaration, message)) in [
        (
            stamped_unit(),
            json!({}),
            "missing-clock-selection",
            "plant-ms",
            Some(("T", STAMPED)),
            format!(
                "clock \"plant-ms\" used by T under {STAMPED} has no configuration; \
                 add \"plant-ms\" to the request's clocks member"
            ),
        ),
        (
            stamped_unit(),
            unused,
            "unused-clock-selection",
            "spare",
            None,
            "the clocks member \"spare\" names a clock no temporal declaration uses".to_owned(),
        ),
        (
            event_unit(),
            mismatched,
            "clock-kind-mismatch",
            "events",
            Some(("E", EVENTS)),
            format!(
                "clock \"events\" is a timestamped_event configuration but E uses \
                 {EVENTS}, which needs event_position"
            ),
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let output = compile(
            directory.path(),
            &unit,
            &format!("compile-{index}.json"),
            selection,
        );
        assert_eq!(output.status.code(), Some(20));
        let envelope = refusal(&output);
        assert_eq!(envelope["stage"], "emit", "{envelope}");
        assert_eq!(envelope["code"], "invalid-request", "{envelope}");
        assert_eq!(envelope["cause"], cause, "{envelope}");
        assert_eq!(envelope["details"]["cause"], cause, "{envelope}");
        assert_eq!(envelope["details"]["clock"], clock, "{envelope}");
        assert_eq!(envelope["message"], message.as_str(), "{envelope}");
        match declaration {
            Some((name, profile)) => {
                assert_eq!(envelope["details"]["declaration"], name, "{envelope}");
                assert_eq!(envelope["details"]["profile"], profile, "{envelope}");
                let span = &envelope["details"]["span"];
                let start = span["start"]["byte"].as_u64().expect("a located span");
                let end = span["end"]["byte"].as_u64().expect("a located span");
                let located = unit
                    .get(usize::try_from(start).unwrap()..usize::try_from(end).unwrap())
                    .expect("a span inside the unit");
                assert_eq!(located, format!("\"{clock}\""), "{envelope}");
            }
            None => {
                assert!(
                    envelope["details"].get("declaration").is_none(),
                    "{envelope}"
                );
                assert!(envelope["details"]["span"].is_null(), "{envelope}");
            }
        }
    }
}

/// FR-640-AC-4, AC-6 (TC-1661): an unknown `kind`, a zero `period` and a
/// member the configuration does not define refuse at stage `request`
/// naming the member; a member named `model-steps` or `model-time` refuses
/// `reserved-clock-name` at stage `emit` whatever the unit declares.
#[test]
#[trace("TC-1661", "FR-640-AC-4", "FR-640-AC-6")]
fn malformed_and_reserved_members_refuse() {
    let directory = tempfile::tempdir().unwrap();
    let mut unknown = event_clocks();
    unknown["events"] = json!({"kind": "wall_clock", "sequence_authority": "events"});
    let mut zero = sample_clocks();
    zero["sample"]["period"] = json!({"kind": "integer", "decimal": "0"});
    let mut extra = stamped_clocks();
    extra["plant-ms"]["epoch"] = json!({"kind": "integer", "decimal": "0"});
    for (index, (unit, selection, member)) in [
        (event_unit(), unknown, "events"),
        (sample_unit(), zero, "sample"),
        (stamped_unit(), extra, "plant-ms"),
    ]
    .into_iter()
    .enumerate()
    {
        let output = compile(
            directory.path(),
            &unit,
            &format!("malformed-{index}.json"),
            selection,
        );
        assert_eq!(output.status.code(), Some(20));
        let envelope = refusal(&output);
        assert_eq!(envelope["stage"], "request", "{envelope}");
        assert_eq!(envelope["code"], "invalid-request", "{envelope}");
        assert!(
            envelope["message"]
                .as_str()
                .is_some_and(|message| message.contains(&format!("clocks member {member:?}"))),
            "{envelope}"
        );
    }
    for (index, name) in ["model-steps", "model-time"].into_iter().enumerate() {
        let mut reserved = event_clocks();
        reserved[name] = json!({"kind": "event_position", "sequence_authority": "events"});
        let output = compile(
            directory.path(),
            &event_unit(),
            &format!("reserved-{index}.json"),
            reserved,
        );
        assert_eq!(output.status.code(), Some(20));
        let envelope = refusal(&output);
        assert_eq!(envelope["stage"], "emit", "{envelope}");
        assert_eq!(envelope["code"], "invalid-request", "{envelope}");
        assert_eq!(
            envelope["details"]["cause"], "reserved-clock-name",
            "{envelope}"
        );
        assert_eq!(envelope["details"]["clock"], name, "{envelope}");
    }
}

/// FR-640-AC-3 (TC-1661): an event-position clause over a model subject
/// with clock `model-steps` compiles with no `clocks` member; its row holds
/// the derived `event_position` configuration naming the subject type's
/// declaration identity text.
#[test]
#[trace("TC-1661", "FR-640-AC-3", "FSpec-FR-540-AC-2")]
fn a_model_steps_clause_compiles_without_clocks_and_derives_its_sequence_authority() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::copy(
        "examples/config-version/model.semantic-ir.json",
        directory.path().join("m.json"),
    )
    .unwrap();
    std::fs::write(
        directory.path().join("t.native"),
        format!(
            "language \"ix:native\" edition \"1-draft\";\n\
             profile T = \"{EVENTS}\";\n\
             model Config = \"example/config-version\";\n\
             temporal Bounded using T over (c: Config::ConfigVersion) clock \"model-steps\" \
             on origin {{ eventually[0,200] holds(c.versionNumber = 2) }}\n"
        ),
    )
    .unwrap();
    let request = json!({"format": "native-compile/1", "request": {
        "models": [{"format": "semantic-ir/2.0.0", "source": {"file": "m.json",
            "authority": "a", "identity": "a:m", "document": "M", "formal_revision": 1}}],
        "program": {"source": {"file": "t.native", "authority": "a", "identity": "a:t",
            "document": "U", "formal_revision": 1}}}});
    let path = directory.path().join("compile.json");
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    let rows = rows(&compile_file(&path));
    let [row] = rows.as_slice() else {
        panic!("one binding row: {rows:?}");
    };
    assert_eq!(row["clock_name"], "model-steps");
    assert_eq!(
        row["clock"],
        json!({"kind": "event_position", "sequence_authority":
            "{\"node\":\"ix://example/config-version/ConfigVersion\",\"package\":\"example/config-version\"}"})
    );
}

/// FR-640, FR-100: a function run of a unit with trace-subject clocks
/// compiles under the request's `clocks` and completes; without them the
/// run refuses `missing-clock-selection` at stage `emit` before any call.
#[test]
#[trace("TC-1660", "FR-640-AC-2")]
fn a_run_request_compiles_under_its_clocks() {
    let directory = tempfile::tempdir().unwrap();
    let run = |name: &str, clocks: Option<Value>| {
        let mut request = json!({"format": "native-run/1", "request": {
            "models": [], "program": program(),
            "call": {"function": "seven", "arguments": []}}});
        if let Some(clocks) = clocks {
            request["request"]["clocks"] = clocks;
        }
        run_file(&write(directory.path(), &event_unit(), name, &request))
    };
    let completed = run("with.json", Some(event_clocks()));
    assert_eq!(
        completed.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&completed.stderr)
    );
    let document: Value = serde_json::from_slice(&completed.stdout).expect("a result");
    assert_eq!(
        document["outcome"],
        json!({"kind": "completed", "value": {"kind": "integer", "decimal": "7"}}),
        "{document}"
    );
    let output = run("without.json", None);
    assert_eq!(output.status.code(), Some(20));
    let envelope = refusal(&output);
    assert_eq!(envelope["stage"], "emit", "{envelope}");
    assert_eq!(envelope["code"], "invalid-request", "{envelope}");
    assert_eq!(
        envelope["details"]["cause"], "missing-clock-selection",
        "{envelope}"
    );
}
