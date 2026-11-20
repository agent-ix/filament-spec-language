// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2477: every verb's machine document validates against its published
//! schema (each verb module asserts that on every outcome through
//! [`super::Outcome::expect`] and [`super::Stream::completed`]); here, each
//! verb's document is byte-identical for a repeated request (FR-286-AC-3,
//! AC-26) and for the request written with every object's members reversed
//! (FR-286-AC-27), for an admitted request and a single-defect refusal.

use fsl_command::command::{RequestOverrides, RunLimits};
use ix_trace_rs::trace;
use serde_json::{json, Value};

use super::{analyze, compile, export, monitor, run, Case};

/// `value` as JSON text with the members of every object in reverse order.
fn reversed(value: &Value) -> String {
    match value {
        Value::Object(members) => {
            let inner: Vec<String> = members
                .iter()
                .rev()
                .map(|(name, member)| {
                    format!("{}:{}", Value::String(name.clone()), reversed(member))
                })
                .collect();
            format!("{{{}}}", inner.join(","))
        }
        Value::Array(items) => {
            let inner: Vec<String> = items.iter().map(reversed).collect();
            format!("[{}]", inner.join(","))
        }
        scalar => scalar.to_string(),
    }
}

/// The request `value` written to `case` twice: in member order as
/// `forward.json` and reversed as `reversed.json`.
fn both_orders(case: &Case, value: &Value) -> (std::path::PathBuf, std::path::PathBuf) {
    let forward = case.json("forward.json", value);
    let backward = case.write("reversed.json", reversed(value).as_bytes());
    assert_ne!(
        std::fs::read(&forward).unwrap(),
        std::fs::read(&backward).unwrap(),
        "the reversed request differs in bytes"
    );
    (forward, backward)
}

fn assert_same(first: &super::Outcome, second: &super::Outcome) {
    assert_eq!(first.exit, second.exit, "{}", first.document);
    assert_eq!(
        String::from_utf8_lossy(&first.bytes),
        String::from_utf8_lossy(&second.bytes)
    );
}

const VALUE_UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"filament.value.complete/v1\";\n\
    record Parcel { pieces: Int[0, 31]; }\n\
    function Remaining using v(total: Integer, held: Integer): Integer pure { total - held }\n";

fn program() -> Value {
    json!({"source": {"file": "unit.fsl", "authority": "machine-output",
        "identity": "machine:unit", "document": "Machine", "formal_revision": 1}})
}

/// FR-286-AC-3, AC-26, AC-27 for `compile`: success and a single-defect
/// (missing source file) refusal are deterministic and order-free.
#[test]
#[trace("FR-286-AC-3", "FR-286-AC-26", "FR-286-AC-27")]
fn compile_documents_are_deterministic_and_order_free() {
    let case = Case::new();
    case.write("unit.fsl", VALUE_UNIT.as_bytes());
    let request = json!({"format": "native-compile/1",
        "request": {"models": [], "program": program()}});
    let mut refused = request.clone();
    refused["request"]["program"]["source"]["file"] = json!("absent.fsl");
    for (value, exit) in [(request, 0), (refused, 20)] {
        let (forward, backward) = both_orders(&case, &value);
        let first = compile(&forward, None, &RequestOverrides::default());
        assert_eq!(first.exit, exit, "{}", first.document);
        super::assert_outcome_schema(&first.document);
        assert_same(
            &first,
            &compile(&forward, None, &RequestOverrides::default()),
        );
        let other = compile(&backward, None, &RequestOverrides::default());
        assert_same(&first, &other);
        assert_eq!(first.package, other.package);
    }
}

/// FR-286-AC-27 for `run`: a completed call and an unknown-parameter
/// refusal are deterministic and order-free.
#[test]
#[trace("FR-286-AC-3", "FR-286-AC-26", "FR-286-AC-27")]
fn run_documents_are_deterministic_and_order_free() {
    let case = Case::new();
    case.write("unit.fsl", VALUE_UNIT.as_bytes());
    let call = |parameter: &str| {
        json!({"format": "native-run/1", "request": {"models": [], "program": program(),
            "call": {"function": "Remaining", "arguments": [
                {"parameter": "total", "value": {"integer": "17"}},
                {"parameter": parameter, "value": {"integer": "5"}}]}}})
    };
    for (value, exit) in [(call("held"), 0), (call("zz"), 20)] {
        let (forward, backward) = both_orders(&case, &value);
        let go =
            |path: &std::path::Path| run(path, RunLimits::default(), &RequestOverrides::default());
        let first = go(&forward);
        assert_eq!(first.exit, exit, "{}", first.document);
        super::assert_outcome_schema(&first.document);
        assert_same(&first, &go(&forward));
        assert_same(&first, &go(&backward));
    }
}

const COUNTER_MODEL: &[u8] = include_bytes!("../../fixtures/cli-contract/counter.semantic-ir.json");

const COUNTER_UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile values = \"filament.value.complete/v1\";\n\
    profile paths = \"filament.temporal.infinite-trace/v1\";\n\
    model Queue = \"test/counter\";\n\
    pre SpaceAvailable using values on Queue::Counter::inc { self.value < 2 }\n\
    post AppendOne using values on Queue::Counter::inc { self.value - pre(self.value) = 1 }\n\
    temporal NeverFull using paths over (q: Queue::Counter) clock \"model-steps\" on origin {\n\
      always holds(q.value < 2)\n\
    }\n";

/// FR-286-AC-26, AC-27 for `analyze`: a refuted item with its
/// counterexample, and a missing-claim refusal, are deterministic and
/// order-free.
#[test]
#[trace("FR-286-AC-3", "FR-286-AC-26", "FR-286-AC-27")]
fn analyze_documents_are_deterministic_and_order_free() {
    let case = Case::new();
    case.write("unit.fsl", COUNTER_UNIT.as_bytes());
    case.write("counter.json", COUNTER_MODEL);
    case.json(
        "start.json",
        &json!({"format": "filament.state.snapshot/v1",
            "identity": {"authority": "contracts", "identity": "queue-start"},
            "observation": "current", "model": {"identity": "test/counter"},
            "anchor": {"kind": "initialization", "name": "queue-start"},
            "populations": [{"population": "ix://test/counter/counters", "complete": true,
                "objects": [{"key": "queue-east", "type": "ix://test/counter/Counter",
                    "fields": {"value": {"integer": "0"}}}]}]}),
    );
    let request = |claim: &str| {
        json!({"format": "native-analyze/1", "request": {"program": program(),
            "models": [{"format": "semantic-ir/2.0.0", "source": {"file": "counter.json",
                "authority": "contracts", "identity": "counter-reference",
                "document": "CounterInput", "formal_revision": 1}}],
            "documents": [{"file": "start.json"}], "items": [{"claim": claim}]}})
    };
    for (value, exit) in [(request("NeverFull"), 10), (request("Absent"), 20)] {
        let (forward, backward) = both_orders(&case, &value);
        let first = analyze(&forward);
        assert_eq!(first.exit, exit, "{}", first.document);
        super::assert_outcome_schema(&first.document);
        assert_same(&first, &analyze(&forward));
        assert_same(&first, &analyze(&backward));
    }
}

/// FR-286-AC-22, AC-26, AC-27 for `export`: an assembled export and an
/// unknown-limit refusal are deterministic and order-free.
#[test]
#[trace("FR-286-AC-22", "FR-286-AC-26", "FR-286-AC-27")]
fn export_documents_are_deterministic_and_order_free() {
    let case = Case::new();
    let request = |limits: Option<Value>| {
        let mut member = super::export::member();
        if let Some(limits) = limits {
            member["limits"] = limits;
        }
        super::export::counter_request(&case, super::export::COUNTER.as_bytes(), member)
    };
    |limits: Option<Value>| {
        let mut member = json!({"profile": "filament.output.tla2/v1",
            "obligations": ["SlowEnough"], "minimum_fidelity": "any"});
        if let Some(limits) = limits {
            member["limits"] = limits;
        }
        json!({"format": "native-run/1", "request": {"models": [], "program": program(),
            "export": member}})
    };
    for (value, exit) in [
        (request(None), 0),
        (request(Some(json!({"output_bytes": 1}))), 20),
    ] {
        let (forward, backward) = both_orders(&case, &value);
        let go = |path: &std::path::Path| export(path, &RequestOverrides::default());
        let first = go(&forward);
        assert_eq!(first.exit, exit, "{}", first.document);
        super::assert_outcome_schema(&first.document);
        assert_same(&first, &go(&forward));
        assert_same(&first, &go(&backward));
    }
}

/// FR-644-AC-12, FR-286-AC-27 for `monitor`: the stream's lines are
/// byte-identical for a repeated run and for the reversed request.
#[test]
#[trace("FR-644-AC-12", "FR-286-AC-27")]
fn monitor_lines_are_deterministic_and_order_free() {
    let case = Case::new();
    case.write(
        "unit.fsl",
        b"language \"ix:native\" edition \"1-draft\";\n\
          profile t = \"filament.temporal.infinite-trace/v1\";\n\
          record Load { bins: Int[0, 15]; }\n\
          temporal StaysLow using t over (s: Load) clock \"steps\" on origin { always holds(s.bins <= 6) }\n",
    );
    let request = json!({"format": "native-monitor/1", "request": {"models": [],
        "program": program(), "clauses": [{"name": "StaysLow"}], "trace": {"kind": "finite"}}});
    let (forward, backward) = both_orders(&case, &request);
    let records = b"{\"observation\":{\"sample\":{\"bins\":{\"integer\":\"2\"}}}}\n\
        {\"observation\":{\"sample\":{\"bins\":{\"integer\":\"9\"}}}}\n{\"end\":true}\n";
    let first = monitor(&forward, records);
    first.completed(10);
    assert_eq!(first.bytes(), monitor(&forward, records).bytes());
    assert_eq!(first.bytes(), monitor(&backward, records).bytes());
}
