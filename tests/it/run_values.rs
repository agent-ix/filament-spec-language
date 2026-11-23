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
function make_enum using v(): Color pure { Color::Green }
function make_record using v(): P pure { P{x: 1, y: 2} }
function make_tuple using v(): Tp pure { Tp(2, 1) }
"#;

fn program_source() -> Value {
    json!({"file":"program.native","authority":"agent-ix","identity":"test:run-values",
        "document":"RunValues","formal_revision":1})
}

/// Run `function` of the identity fixture with one argument `a = value`.
fn run_identity(function: &str, value: Value) -> Output {
    run_fixture(function, json!([{"parameter":"a","value":value}]))
}

fn run_fixture(function: &str, arguments: Value) -> Output {
    let directory = tempfile::tempdir().unwrap();
    write_request(directory.path(), function, arguments);
    run_file(&directory.path().join("request.json"))
}

fn write_request(directory: &Path, function: &str, arguments: Value) {
    std::fs::write(directory.join("program.native"), IDENTITY_FIXTURE).unwrap();
    let request = json!({"format":"native-run/1","request":{
        "models":[],
        "program":{"source":program_source()},
        "call":{"function":function,"arguments":arguments},
    }});
    std::fs::write(
        directory.join("request.json"),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();
}

/// Read the canonical declaration name from the actual fixture's writer.
fn fixture_name(function: &str) -> Value {
    let output = run_fixture(function, json!([]));
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    let name = document["outcome"]["value"]["name"].clone();
    assert!(name.is_string(), "{document}");
    name
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
fn boolean_round_trips() {
    assert_round_trip(
        "id_bool",
        json!({"type": "boolean", "value": true}),
        json!({"type": "boolean", "value": true}),
    );
    assert_round_trip("id_bool", json!({"type": "boolean", "value": true}), json!({"type": "boolean", "value": true}));
}

#[test]
#[trace("TC-1700", "FR-650-AC-1")]
fn integer_round_trips() {
    let big = "1267650600228229401496703205376";
    assert_round_trip("id_big", json!({"type": "integer", "value": big}), json!({"type": "integer", "value": big}));
    assert_round_trip("id_big", json!({"type": "integer", "value": "7"}), json!({"type": "integer", "value": "7"}));
}

#[test]
#[trace("TC-1700", "FR-650-AC-1")]
fn rational_round_trips_normalized() {
    assert_round_trip(
        "id_rat",
        json!({"type": "rational", "numerator": "4", "denominator": "6"}),
        json!({"type": "rational", "numerator": "2", "denominator": "3"}),
    );
}

#[test]
#[trace("TC-1700", "FR-650-AC-1")]
fn decimal_round_trips_retained() {
    let value = json!({"type": "decimal", "coefficient": "150", "scale": "2"});
    assert_round_trip("id_dec", value.clone(), value);
}

#[test]
#[trace("TC-1700", "FR-650-AC-1")]
fn float_round_trips_by_bits() {
    let value = json!({"type": "float64", "bits": "3ff0000000000000"});
    assert_round_trip("id_flt", value.clone(), value);
}

#[test]
#[trace("TC-1700", "FR-650-AC-1")]
fn string_round_trips() {
    let value = json!({"type": "string", "value": "abc"});
    assert_round_trip("id_str", value.clone(), value);
}

#[test]
#[trace("TC-1700", "FR-650-AC-1")]
fn text_round_trips_normalized() {
    assert_round_trip(
        "id_txt",
        json!({"type": "text", "value": "e\u{0301}"}),
        json!({"type": "text", "value": "\u{00e9}"}),
    );
}

#[test]
#[trace("TC-1700", "FR-650-AC-1")]
fn enum_round_trips() {
    let value = json!({"type": "enum", "name": fixture_name("make_enum"), "member": "Green"});
    assert_round_trip("id_enum", value.clone(), value);
}

#[test]
#[trace("TC-1700", "FR-650-AC-1")]
fn record_round_trips() {
    let value =
        json!({"type": "record", "name": fixture_name("make_record"), "fields": [{"name": "x", "value": {"type": "integer", "value": "1"}}, {"name": "y", "value": {"type": "integer", "value": "2"}}]});
    assert_round_trip("id_rec", value.clone(), value);
}

#[test]
#[trace("TC-1700", "FR-650-AC-1")]
fn tuple_round_trips() {
    let value =
        json!({"type": "tuple", "name": fixture_name("make_tuple"), "components": [{"type": "integer", "value": "2"}, {"type": "integer", "value": "1"}]});
    assert_round_trip("id_tup", value.clone(), value);
}

#[test]
#[trace("TC-1700", "FR-650-AC-1")]
fn sequence_round_trips() {
    let value = json!({"type": "sequence", "elements": [{"type": "integer", "value": "1"}, {"type": "integer", "value": "1"}]});
    assert_round_trip("id_seq", value.clone(), value);
}

#[test]
#[trace("TC-1700", "FR-650-AC-1")]
fn set_round_trips_in_canonical_order() {
    assert_round_trip(
        "id_set",
        json!({"type": "set", "elements": [{"type": "integer", "value": "3"}, {"type": "integer", "value": "1"}]}),
        json!({"type": "set", "elements": [{"type": "integer", "value": "1"}, {"type": "integer", "value": "3"}]}),
    );
}

#[test]
#[trace("TC-1700", "FR-650-AC-1")]
fn option_round_trips() {
    let present = json!({"type": "option", "value": {"type": "integer", "value": "3"}});
    assert_round_trip("id_opt", present.clone(), present);
    assert_round_trip("id_opt", json!({"type": "option", "value": null}), json!({"type": "option", "value": null}));
}

#[test]
#[trace("TC-1701", "FR-650-AC-2")]
fn integer_above_i64_passes_as_a_tagged_decimal_string() {
    assert_round_trip(
        "id_big",
        json!({"type": "integer", "value": "9223372036854775808"}),
        json!({"type": "integer", "value": "9223372036854775808"}),
    );
}

/// Negative: a bare JSON integer above i64, and other non-object values,
/// refuse at the request stage naming the value.
#[test]
#[trace("TC-1701", "FR-650-AC-2")]
fn integer_above_i64_in_the_wrong_form_refuses_at_the_request() {
    for value in [
        json!(9_223_372_036_854_775_808_u64),
        json!(1),
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
fn mistyped_arguments_refuse_with_cause_and_path() {
    let record = |fields: Value| json!({"type": "record", "name": fixture_name("make_record"), "fields": fields});
    assert_call_refusal(
        "id_str",
        json!({"type": "text", "value": "a"}),
        "invalid_runtime_input",
        "wrong-value-kind",
        "",
    );
    assert_call_refusal(
        "id_rec",
        record(json!([{"name": "x", "value": {"type": "integer", "value": "1"}}])),
        "invalid_runtime_input",
        "invalid-value",
        "/fields",
    );
    assert_call_refusal(
        "id_rec",
        record(json!([{"name": "x", "value": {"type": "integer", "value": "1"}}, {"name": "y", "value": {"type": "integer", "value": "2"}}, {"name": "z", "value": {"type": "integer", "value": "3"}}])),
        "invalid_runtime_input",
        "invalid-value",
        "/fields",
    );
    assert_call_refusal(
        "id_enum",
        json!({"type": "enum", "name": fixture_name("make_enum"), "member": "Blue"}),
        "invalid_runtime_input",
        "invalid-value",
        "/member",
    );
    assert_call_refusal(
        "id_rat",
        json!({"type": "rational", "numerator": "1", "denominator": "0"}),
        "invalid_runtime_input",
        "invalid-value",
        "",
    );
    assert_call_refusal(
        "id_set",
        json!({"type": "set", "elements": [{"type": "integer", "value": "1"}, {"type": "integer", "value": "1"}]}),
        "invalid_runtime_input",
        "duplicate-member",
        "/elements/1",
    );
    assert_call_refusal(
        "id_seq",
        json!({"type": "sequence", "elements": [{"type": "integer", "value": "1"}, {"type": "integer", "value": "1"}, {"type": "integer", "value": "1"}, {"type": "integer", "value": "1"}]}),
        "cardinality_out_of_bound",
        "above-maximum",
        "",
    );
    assert_call_refusal(
        "id_tup",
        json!({"type": "tuple", "name": fixture_name("make_tuple"), "components": [{"type": "integer", "value": "2"}]}),
        "invalid_runtime_input",
        "invalid-value",
        "/components",
    );
    assert_call_refusal(
        "id_bool",
        json!({"type": "integer", "value": "2"}),
        "invalid_runtime_input",
        "wrong-value-kind",
        "",
    );
}

/// FR-650-AC-6: no identity-fixture function refuses as unsupported; the
/// unsupported-type refusal names the function.
#[test]
#[trace("TC-1705", "FR-650-AC-6")]
fn no_identity_function_refuses_unsupported() {
    for (function, argument) in [
        (
            "id_rec",
            json!({"type": "record", "name": fixture_name("make_record"), "fields": [{"name": "x", "value": {"type": "integer", "value": "1"}}, {"name": "y", "value": {"type": "integer", "value": "2"}}]}),
        ),
        (
            "id_tup",
            json!({"type": "tuple", "name": fixture_name("make_tuple"), "components": [{"type": "integer", "value": "2"}, {"type": "integer", "value": "1"}]}),
        ),
        (
            "id_enum",
            json!({"type": "enum", "name": fixture_name("make_enum"), "member": "Red"}),
        ),
        ("id_seq", json!({"type": "sequence", "elements": []})),
        ("id_set", json!({"type": "set", "elements": []})),
        ("id_txt", json!({"type": "text", "value": "a"})),
    ] {
        let output = run_identity(function, argument);
        assert_eq!(output.status.code(), Some(0), "{function}: {output:?}");
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
        json!({"kind": "completed", "value": {"type": "integer", "value": "7"}})
    );
}

/// FSpec FR-550-AC-1: argument and completed values of each kind keep one
/// typed form, normalized where the form normalizes.
#[test]
#[trace("FSpec-TC-1200", "FSpec-FR-550-AC-1")]
fn fspec_values_round_trip_in_one_form() {
    let big = "1267650600228229401496703205376";
    assert_round_trip("id_big", json!({"type": "integer", "value": big}), json!({"type": "integer", "value": big}));
    assert_round_trip(
        "id_rat",
        json!({"type": "rational", "numerator": "4", "denominator": "6"}),
        json!({"type": "rational", "numerator": "2", "denominator": "3"}),
    );
    assert_round_trip(
        "id_set",
        json!({"type": "set", "elements": [{"type": "integer", "value": "3"}, {"type": "integer", "value": "1"}]}),
        json!({"type": "set", "elements": [{"type": "integer", "value": "1"}, {"type": "integer", "value": "3"}]}),
    );
    let tuple =
        json!({"type": "tuple", "name": fixture_name("make_tuple"), "components": [{"type": "integer", "value": "2"}, {"type": "integer", "value": "1"}]});
    assert_round_trip("id_tup", tuple.clone(), tuple);
}

/// FSpec FR-550-AC-2: an argument that is not a value of its declared type
/// refuses with its cause, naming the parameter and the location.
#[test]
#[trace("FSpec-TC-1201", "FSpec-FR-550-AC-2")]
fn fspec_mistyped_arguments_refuse_with_cause_and_location() {
    assert_call_refusal(
        "id_rec",
        json!({"type": "record", "name": fixture_name("make_record"), "fields": [{"name": "x", "value": {"type": "integer", "value": "1"}}]}),
        "invalid_runtime_input",
        "invalid-value",
        "/fields",
    );
    assert_call_refusal(
        "id_set",
        json!({"type": "set", "elements": [{"type": "integer", "value": "1"}, {"type": "integer", "value": "1"}]}),
        "invalid_runtime_input",
        "duplicate-member",
        "/elements/1",
    );
}

/// FR-100-AC-21: a completed function run's result document has no `axes`.
#[test]
#[trace("TC-1582", "FR-100-AC-21")]
fn completed_run_document_carries_no_axes() {
    let output = run_identity("id_bool", json!({"type": "boolean", "value": true}));
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(document.get("axes").is_none(), "{document}");
}

/// A unit's source: `Quantity` has a run value form (FR-650).
const QUANTITY_FIXTURE: &str = r#"language "ix:native" edition "1-draft";
profile v = "filament.value.complete/v1";
dimension Time;
unit s: Time = rational(1, 1);
function take using v(a: Quantity<s>): Boolean pure { true }
function give using v(): Quantity<s> pure { 2 s }
"#;

fn run_quantity(function: &str, arguments: Value) -> Output {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("program.native"), QUANTITY_FIXTURE).unwrap();
    let request = json!({"format":"native-run/1","request":{
        "models":[],
        "program":{"source":program_source()},
        "call":{"function":function,"arguments":arguments},
    }});
    std::fs::write(
        directory.path().join("request.json"),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();
    run_file(&directory.path().join("request.json"))
}

/// FR-650-AC-10: `Quantity` has a run value form. A bare integer refuses
/// at the request, an Integer value refuses as the wrong kind at the call,
/// and a quantity result completes. A type with no form (a collection over a
/// population) is covered by the `run_value` unit tests.
#[test]
#[trace("TC-1705", "FR-650-AC-6")]
fn quantity_has_a_form_so_it_is_not_refused_unsupported() {
    for (value, stage, code, cause) in [
        (json!(2), "request", "invalid-request", None),
        (json!({"type": "integer", "value": "2"}), "call", "invalid_runtime_input", Some("wrong-value-kind")),
    ] {
        let output = run_quantity("take", json!([{"parameter":"a","value":value}]));
        assert_eq!(output.status.code(), Some(20), "{output:?}");
        let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(failure["stage"], stage, "{failure}");
        assert_eq!(failure["code"], code, "{failure}");
        if let Some(cause) = cause {
            assert_eq!(failure["cause"], cause, "{failure}");
        } else {
            assert!(failure.to_string().contains("/call/arguments/0/value"), "{failure}");
        }
    }
    let output = run_quantity("give", json!([]));
    assert_eq!(output.status.code(), Some(0), "{output:?}");
}
