// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-933-AC-2 and AC-3 (TC-2606): a `timed` configuration fixes a
//! role-clock timed clause's binding unit; unit-free bounds are magnitudes in
//! it, unit-bearing bounds convert exactly to it, and a unit that is not a
//! declared time unit is refused at stage `emit`.

use crate::command_harness::{compile_file, Output};
use ix_trace_rs::trace;
use serde_json::{json, Value};

fn unit(bounds: &str, extra: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         profile t = \"filament.temporal.timed/v1\";\n\
         dimension Time;\n\
         unit s : Time = rational(1, 1);\n\
         unit ms : Time = rational(1, 1000) * s;\n\
         unit us : Time = rational(1, 1000000) * s;\n\
         {extra}\
         record Reading {{ req: Boolean; ack: Boolean; }}\n\
         temporal Ack using t over (r: Reading) clock \"plant\" on origin \
         {{ always (holds(r.req) implies eventually{bounds} holds(r.ack)) }}\n"
    )
}

fn compile(unit: &str, clock_unit: &str) -> Output {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("unit.native"), unit).unwrap();
    let request = json!({"format": "native-compile/1", "request": {
        "models": [],
        "program": {"source": {"file": "unit.native", "authority": "agent-ix",
            "identity": "test:timed-clock-unit", "document": "TimedClockUnit",
            "formal_revision": 1}},
        "clocks": {"plant": {"kind": "timed", "unit": clock_unit}}}});
    let path = directory.path().join("compile.json");
    std::fs::write(&path, request.to_string()).unwrap();
    compile_file(&path)
}

/// Every interval bound magnitude the emitted package writes, as text.
fn bounds(output: &Output) -> Vec<String> {
    let text = String::from_utf8(output.stdout.clone()).expect("UTF-8 package");
    let package: Value = serde_json::from_str(&text).expect("a JSON package");
    let mut found = Vec::new();
    collect(&package, &mut found);
    found
}

fn collect(value: &Value, found: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            if let (Some(lower), Some(upper)) = (map.get("lower"), map.get("upper")) {
                found.push(format!("{lower}..{upper}"));
            }
            map.values().for_each(|child| collect(child, found));
        }
        Value::Array(items) => items.iter().for_each(|child| collect(child, found)),
        _ => {}
    }
}

fn refusal(output: &Output) -> Value {
    serde_json::from_slice(&output.stderr).expect("a refusal envelope")
}

#[test]
#[trace("TC-2606", "FR-933-AC-2", "FR-933-AC-3")]
fn a_timed_configuration_fixes_the_binding_unit() {
    let bare = compile(&unit("[0, 200]", ""), "ms");
    let named = compile(&unit("[0 ms, 200 ms]", ""), "ms");
    assert_eq!(
        bare.status.code(),
        Some(0),
        "{:?}",
        String::from_utf8_lossy(&bare.stderr)
    );
    assert_eq!(named.status.code(), Some(0));
    assert_eq!(bounds(&bare), bounds(&named), "equal bounds in ms");

    let micro = compile(&unit("[0 ms, 200 ms]", ""), "us");
    assert_eq!(micro.status.code(), Some(0));
    assert!(
        bounds(&micro).iter().any(|bound| bound.contains("200000")),
        "200 ms is 200000 us: {:?}",
        bounds(&micro)
    );
    assert_ne!(
        bounds(&micro),
        bounds(&named),
        "a different binding unit, a different key"
    );

    let unknown = compile(&unit("[0 ms, 200 ms]", ""), "minute");
    assert_eq!(unknown.status.code(), Some(20));
    let envelope = refusal(&unknown).to_string();
    assert!(
        envelope.contains("invalid-request") && envelope.contains("unknown-clock-unit"),
        "{envelope}"
    );
    assert!(
        envelope.contains("plant") && envelope.contains("minute"),
        "{envelope}"
    );

    let length = "dimension Length;\nunit m : Length = rational(1, 1);\n";
    let wrong = compile(&unit("[0 ms, 200 ms]", length), "m");
    assert_eq!(wrong.status.code(), Some(20));
    assert!(refusal(&wrong).to_string().contains("clock-unit-dimension"));

    let bound = compile(&unit("[0 m, 200 m]", length), "ms");
    assert_eq!(bound.status.code(), Some(20));
    let envelope = refusal(&bound).to_string();
    assert!(envelope.contains("type-mismatch"), "{envelope}");
}
