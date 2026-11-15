// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-650: every argument and result kind of a `1-draft` function run is
//! carried in the run value form, through `command`'s `run`.
//!
//! The identity fixture's functions return their argument, so one call
//! checks the argument form and the result form of one kind.

use crate::command_harness::{run_file, Output};
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::path::Path;

/// FR-650's identity fixture.
const IDENTITY_FIXTURE: &str = r#"language "ix:native" edition "1-draft";
profile v = "filament.value.complete/v1";
type Big = Int[0, 1267650600228229401496703205376];
type R = Rational[0, 9; 1, 9];
type D = Decimal[0, 1000; 0, 2; exact];
type F = Float64[nearest-even];
type T = Text[0, 8; nfc];
enum Color { Red, Green }
record P { x: Int[0, 9]; y: Int[0, 9]; }
tuple Tp(Int[0, 5], Int[0, 5]);
function id_bool using v(a: Boolean): Boolean pure { a }
function id_big using v(a: Big): Big pure { a }
function id_rat using v(a: R): R pure { a }
function id_dec using v(a: D): D pure { a }
function id_flt using v(a: F): F pure { a }
function id_str using v(a: String): String pure { a }
function id_txt using v(a: T): T pure { a }
function id_enum using v(a: Color): Color pure { a }
function id_rec using v(a: P): P pure { a }
function id_tup using v(a: Tp): Tp pure { a }
function id_seq using v(a: Sequence<Int[0, 9]>[0, 3]): Sequence<Int[0, 9]>[0, 3] pure { a }
function id_set using v(a: Set<Int[0, 9]>[0, 3]): Set<Int[0, 9]>[0, 3] pure { a }
function id_opt using v(a: Option<Int[0, 9]>): Option<Int[0, 9]> pure { a }
"#;

fn program_source() -> Value {
    json!({"file":"program.native","authority":"agent-ix","identity":"test:run-values",
        "document":"RunValues","formal_revision":1})
}

/// Run `function` of the identity fixture with one argument `a = value`.
fn run_identity(function: &str, value: Value) -> Output {
    let directory = tempfile::tempdir().unwrap();
    write_request(directory.path(), function, value);
    run_file(&directory.path().join("request.json"))
}

fn write_request(directory: &Path, function: &str, value: Value) {
    std::fs::write(directory.join("program.native"), IDENTITY_FIXTURE).unwrap();
    let request = json!({"format":"native-run/1","request":{
        "models":[],
        "program":{"source":program_source()},
        "call":{"function":function,"arguments":[{"parameter":"a","value":value}]},
    }});
    std::fs::write(
        directory.join("request.json"),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();
}

/// Assert `function(argument)` completes with `expected`, exit 0.
fn assert_round_trip(function: &str, argument: Value, expected: Value) {
    let output = run_identity(function, argument);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{function}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["format"], "spine-run-result/1");
    assert_eq!(
        document["outcome"],
        json!({"kind": "completed", "value": expected}),
        "{function}"
    );
}

/// Assert `function(argument)` refuses at stage `call`, exit 20, with
/// `code`/`cause`, position 0 and `path`.
fn assert_call_refusal(function: &str, argument: Value, code: &str, cause: &str, path: &str) {
    let output = run_identity(function, argument);
    assert_eq!(output.status.code(), Some(20), "{function}");
    assert!(output.stdout.is_empty(), "{function}");
    let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(failure["stage"], "call", "{function}: {failure}");
    assert_eq!(failure["code"], code, "{function}: {failure}");
    assert_eq!(failure["cause"], cause, "{function}: {failure}");
    assert_eq!(failure["details"]["position"], 0, "{function}: {failure}");
    assert_eq!(failure["details"]["path"], path, "{function}: {failure}");
}

#[test]
#[trace("TC-1700", "FR-650-AC-1")]
#[ignore = "QSL-1988: Boolean arguments in the run value form not implemented yet"]
fn boolean_round_trips() {
    assert_round_trip(
        "id_bool",
        json!({"boolean": true}),
        json!({"boolean": true}),
    );
    assert_round_trip("id_bool", json!(1), json!({"boolean": true}));
}

#[test]
#[trace("TC-1700", "FR-650-AC-1")]
#[ignore = "QSL-1988: integer arguments beyond i64 in the run value form not implemented yet"]
fn integer_round_trips() {
    let big = "1267650600228229401496703205376";
    assert_round_trip("id_big", json!({"integer": big}), json!({"integer": big}));
    assert_round_trip("id_big", json!(7), json!({"integer": "7"}));
}

#[test]
#[trace("TC-1700", "FR-650-AC-1")]
#[ignore = "QSL-2182: Rational arguments and results in the run value form not implemented yet"]
fn rational_round_trips_normalized() {
    assert_round_trip(
        "id_rat",
        json!({"kind": "rational", "numerator": "4", "denominator": "6"}),
        json!({"kind": "rational", "numerator": "2", "denominator": "3"}),
    );
}

#[test]
#[trace("TC-1700", "FR-650-AC-1")]
#[ignore = "QSL-2182: Decimal arguments and results in the run value form not implemented yet"]
fn decimal_round_trips_retained() {
    let value = json!({"kind": "decimal", "coefficient": "150", "scale": "2"});
    assert_round_trip("id_dec", value.clone(), value);
}

#[test]
#[trace("TC-1700", "FR-650-AC-1")]
#[ignore = "QSL-2182: Float arguments and results in the run value form not implemented yet"]
fn float_round_trips_by_bits() {
    let value = json!({"kind": "float64", "bits": "3ff0000000000000"});
    assert_round_trip("id_flt", value.clone(), value);
}

#[test]
#[trace("TC-1700", "FR-650-AC-1")]
#[ignore = "QSL-2182: String arguments and results in the run value form not implemented yet"]
fn string_round_trips() {
    let value = json!({"kind": "string", "text": "abc"});
    assert_round_trip("id_str", value.clone(), value);
}

#[test]
#[trace("TC-1700", "FR-650-AC-1")]
#[ignore = "QSL-2182: Text arguments and results in the run value form not implemented yet"]
fn text_round_trips_normalized() {
    assert_round_trip(
        "id_txt",
        json!({"kind": "text", "text": "e\u{0301}"}),
        json!({"kind": "text", "text": "\u{00e9}"}),
    );
}

#[test]
#[trace("TC-1700", "FR-650-AC-1")]
#[ignore = "QSL-2182: enum arguments and results in the run value form not implemented yet"]
fn enum_round_trips() {
    let value = json!({"variant": {"type": "Color", "case": "Green"}});
    assert_round_trip("id_enum", value.clone(), value);
}

#[test]
#[trace("TC-1700", "FR-650-AC-1")]
#[ignore = "QSL-2182: record arguments and results in the run value form not implemented yet"]
fn record_round_trips() {
    let value =
        json!({"record": {"type": "P", "fields": {"x": {"integer": "1"}, "y": {"integer": "2"}}}});
    assert_round_trip("id_rec", value.clone(), value);
}

#[test]
#[trace("TC-1700", "FR-650-AC-1")]
#[ignore = "QSL-2182: tuple arguments and results in the run value form not implemented yet"]
fn tuple_round_trips() {
    let value =
        json!({"tuple": {"type": "Tp", "components": [{"integer": "2"}, {"integer": "1"}]}});
    assert_round_trip("id_tup", value.clone(), value);
}

#[test]
#[trace("TC-1700", "FR-650-AC-1")]
#[ignore = "QSL-2182: Sequence arguments and results in the run value form not implemented yet"]
fn sequence_round_trips() {
    let value = json!({"sequence": [{"integer": "1"}, {"integer": "1"}]});
    assert_round_trip("id_seq", value.clone(), value);
}

#[test]
#[trace("TC-1700", "FR-650-AC-1")]
#[ignore = "QSL-2182: Set arguments and results in the run value form not implemented yet"]
fn set_round_trips_in_canonical_order() {
    assert_round_trip(
        "id_set",
        json!({"set": [{"integer": "3"}, {"integer": "1"}]}),
        json!({"set": [{"integer": "1"}, {"integer": "3"}]}),
    );
}

#[test]
#[trace("TC-1700", "FR-650-AC-1")]
#[ignore = "QSL-2182: Option arguments and results in the run value form not implemented yet"]
fn option_round_trips() {
    let present = json!({"present": {"integer": "3"}});
    assert_round_trip("id_opt", present.clone(), present);
    assert_round_trip("id_opt", json!({"absent": {}}), json!({"absent": {}}));
}

#[test]
#[trace("TC-1701", "FR-650-AC-2")]
#[ignore = "QSL-1988: integers beyond i64 as tagged decimal strings not implemented yet"]
fn integer_above_i64_passes_as_a_tagged_decimal_string() {
    assert_round_trip(
        "id_big",
        json!({"integer": "9223372036854775808"}),
        json!({"integer": "9223372036854775808"}),
    );
}

/// Negative: a bare JSON integer above i64, and other non-object values,
/// refuse at the request stage naming the value.
#[test]
#[trace("TC-1701", "FR-650-AC-2")]
#[ignore = "QSL-1988: request-stage refusal naming the argument value not implemented yet"]
fn integer_above_i64_in_the_wrong_form_refuses_at_the_request() {
    for value in [
        json!(9_223_372_036_854_775_808_u64),
        json!("7"),
        json!(1.5),
        json!(true),
        Value::Null,
        json!([1]),
    ] {
        let output = run_identity("id_big", value.clone());
        assert_eq!(output.status.code(), Some(20), "{value}");
        assert!(output.stdout.is_empty(), "{value}");
        let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(failure["code"], "invalid-request", "{value}: {failure}");
        assert_eq!(failure["stage"], "request", "{value}: {failure}");
        assert!(
            failure.to_string().contains("/call/arguments/0/value"),
            "{value}: {failure}"
        );
    }
}

/// Negative: an argument that is not a value of its declared type.
#[test]
#[trace("TC-1702", "FR-650-AC-3")]
#[ignore = "QSL-1988: typed argument refusals with cause and path not implemented yet"]
fn mistyped_arguments_refuse_with_cause_and_path() {
    let record = |fields: Value| json!({"record": {"type": "P", "fields": fields}});
    assert_call_refusal(
        "id_str",
        json!({"kind": "text", "text": "a"}),
        "invalid_runtime_input",
        "wrong-value-kind",
        "",
    );
    assert_call_refusal(
        "id_rec",
        record(json!({"x": {"integer": "1"}})),
        "invalid_runtime_input",
        "missing-member",
        "/record/fields",
    );
    assert_call_refusal(
        "id_rec",
        record(json!({"x": {"integer": "1"}, "y": {"integer": "2"}, "z": {"integer": "3"}})),
        "invalid_runtime_input",
        "unknown-member",
        "/record/fields/z",
    );
    assert_call_refusal(
        "id_enum",
        json!({"variant": {"type": "Color", "case": "Blue"}}),
        "invalid_runtime_input",
        "invalid-value",
        "/variant/case",
    );
    assert_call_refusal(
        "id_rat",
        json!({"kind": "rational", "numerator": "1", "denominator": "0"}),
        "invalid_runtime_input",
        "invalid-value",
        "",
    );
    assert_call_refusal(
        "id_set",
        json!({"set": [{"integer": "1"}, {"integer": "1"}]}),
        "invalid_runtime_input",
        "duplicate-member",
        "/set/1",
    );
    assert_call_refusal(
        "id_seq",
        json!({"sequence": [{"integer": "1"}, {"integer": "1"}, {"integer": "1"}, {"integer": "1"}]}),
        "cardinality_out_of_bound",
        "above-maximum",
        "",
    );
    assert_call_refusal(
        "id_tup",
        json!({"tuple": {"type": "Tp", "components": [{"integer": "2"}]}}),
        "invalid_runtime_input",
        "invalid-value",
        "/tuple/components",
    );
    assert_call_refusal(
        "id_bool",
        json!(2),
        "invalid_runtime_input",
        "invalid-value",
        "",
    );
}

/// FR-650-AC-6: no identity-fixture function refuses as unsupported; the
/// unsupported-type refusal names the function.
#[test]
#[trace("TC-1705", "FR-650-AC-6")]
#[ignore = "QSL-2182: every result kind with an FR-181 form completes; unsupported_construct removed, not implemented yet"]
fn no_identity_function_refuses_unsupported() {
    for function in ["id_rec", "id_tup", "id_enum", "id_seq", "id_set", "id_txt"] {
        let output = run_identity(function, json!({"absent": {}}));
        let failure: Value = serde_json::from_slice(&output.stderr).unwrap_or(Value::Null);
        assert_ne!(
            failure["code"], "unsupported_construct",
            "{function}: {failure}"
        );
        assert_ne!(output.status.code(), Some(21), "{function}: {failure}");
    }
}

/// FR-650-AC-8: the `execute` outcome carries the run value form.
#[test]
#[trace("TC-1707", "FR-650-AC-8")]
#[ignore = "QSL-2182: execute outcome in the run value form not implemented yet"]
fn seven_completes_as_a_tagged_integer() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(
        directory.path().join("program.native"),
        std::fs::read("tests/fixtures/spine-compile.native").unwrap(),
    )
    .unwrap();
    let request = json!({"format":"native-run/1","request":{
        "models":[],
        "program":{"source":program_source()},
        "call":{"function":"seven","arguments":[]},
    }});
    std::fs::write(
        directory.path().join("request.json"),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();
    let output = run_file(&directory.path().join("request.json"));
    assert_eq!(output.status.code(), Some(0));
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        document["outcome"],
        json!({"kind": "completed", "value": {"integer": "7"}})
    );
}
