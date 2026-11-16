// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-767 (TC-2188): the `integer_division` member of `native-compile/1` and
//! `native-run/1` requests supplies the law for `div` and `rem`.

use std::path::{Path, PathBuf};

use fsl_command::command;
use fsl_replay::Operation;
use ix_trace_rs::trace;
use serde_json::{json, Value};

const HEADER: &str =
    "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\n";
const FLOOR_HEADER: &str = "profile d = \"filament.value.integer-division.floor/v1\";\n";
const BODY: &str = "function q using v(a: Int[-7, 7], b: Int[1, 3]): Int[-7, 7] pure { a div b }\n\
                    function r using v(a: Int[-7, 7], b: Int[1, 3]): Int[-7, 7] pure { a rem b }\n";
const MOD_ONLY: &str =
    "function m using v(a: Int[-7, 7], b: Int[1, 3]): Integer pure { (a mod b) + 0 }\n";
const LIBRARY: &str =
    "function ld using v(a: Int[-7, 7], b: Int[1, 3]): Int[-7, 7] pure { a div b }\n";
const IMPORTING: &str = "import \"test/lib\" as l;\n\
     function q using v(a: Int[-7, 7], b: Int[1, 3]): Int[-7, 7] pure { a div b }\n\
     function viaLib using v(a: Int[-7, 7], b: Int[1, 3]): Int[-7, 7] pure { l::ld(a, b) }\n";

fn source(file: &str, identity: &str) -> Value {
    json!({"file":file, "authority":"agent-ix", "identity":identity,
        "document":"DivisionLaw", "formal_revision":1})
}

fn write(directory: &Path, name: &str, bytes: &[u8]) {
    std::fs::write(directory.join(name), bytes).unwrap();
}

/// A request file holding `format` and `request`, with the program written to
/// `program.native` unless `program` is `None`.
fn request(directory: &Path, program: Option<&str>, format: &str, request: Value) -> PathBuf {
    if let Some(program) = program {
        write(directory, "program.native", program.as_bytes());
    }
    let path = directory.join("request.json");
    let envelope = json!({"format": format, "request": request});
    std::fs::write(&path, serde_json::to_vec(&envelope).unwrap()).unwrap();
    path
}

fn compile_body(law: Option<Value>) -> Value {
    let mut body =
        json!({"models":[], "program":{"source":source("program.native", "test:division")}});
    if let Some(law) = law {
        body["integer_division"] = law;
    }
    body
}

fn run_body(law: Option<Value>, function: &str) -> Value {
    let mut body = compile_body(law);
    body["call"] = json!({"function": function, "arguments":[
        {"parameter":"a","value":-7},{"parameter":"b","value":3}],
        "accounting":{"work_units":100_000}});
    body
}

/// The FR-286 outcome document of a refused command.
fn outcome(error: &command::RunError, operation: Operation) -> Value {
    serde_json::from_slice(&error.outcome_document(operation).to_bytes().unwrap()).unwrap()
}

/// The definition identity the law name `law` selects.
fn law_identity(law: &str) -> String {
    format!("filament.value.integer-division.{law}/v1")
}

/// The law each emitted `div` and `rem` application of `package` binds, as
/// the definition identity it names, in node order.
fn division_laws(package: &[u8]) -> Vec<String> {
    let wire: Value = serde_json::from_slice(package).expect("package JSON");
    wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .filter(|node| {
            matches!(
                node["body"]["operation"]["identity"].as_str(),
                Some("filament.op.integer.div" | "filament.op.integer.rem")
            )
        })
        .map(|node| {
            let definition = &node["body"]["operation"]["laws"][0]["definition"];
            definition["identity"]
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| definition.to_string())
        })
        .collect()
}

/// `(code, cause, message)` of a refused command's first diagnostic.
fn refusal(error: &command::RunError, operation: Operation) -> (String, String, String) {
    let document: Value =
        serde_json::from_slice(&error.outcome_document(operation).to_bytes().unwrap()).unwrap();
    let diagnostic = &document["diagnostics"][0];
    (
        diagnostic["code"].as_str().unwrap_or_default().to_owned(),
        diagnostic["cause"].as_str().unwrap_or_default().to_owned(),
        diagnostic["message"]
            .as_str()
            .unwrap_or_default()
            .to_owned(),
    )
}

/// The integer a completed run returned.
fn returned(path: &Path) -> String {
    let result = command::run(path).expect("the run completes");
    assert_eq!(result.exit_code, 0);
    result.value.as_value()["outcome"]["value"]["decimal"]
        .as_str()
        .map(str::to_owned)
        .or_else(|| {
            result.value.as_value()["outcome"]["value"]["decimal"]
                .as_i64()
                .map(|value| value.to_string())
        })
        .expect("an integer value")
}

#[test]
#[trace("TC-2188", "FR-767-AC-1", "FSpec-TC-1652", "FSpec-FR-570-AC-6")]
fn request_member_selects_each_law_for_compile_and_run() {
    for (law, quotient, remainder) in [
        ("truncating", "-2", "-1"),
        ("floor", "-3", "2"),
        ("euclidean", "-3", "2"),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let program = format!("{HEADER}{BODY}");
        let path = request(
            directory.path(),
            Some(&program),
            "native-compile/1",
            compile_body(Some(json!(law))),
        );
        let bytes =
            command::compile(&path).unwrap_or_else(|error| panic!("{law} compiles: {error}"));
        assert_eq!(division_laws(&bytes), vec![law_identity(law); 2], "{law}");
        for (function, expected) in [("q", quotient), ("r", remainder)] {
            let path = request(
                directory.path(),
                None,
                "native-run/1",
                run_body(Some(json!(law)), function),
            );
            assert_eq!(returned(&path), expected, "{function} under {law}");
        }
    }
}

#[test]
#[trace("TC-2188", "FR-767-AC-2", "FSpec-TC-1652", "FSpec-FR-570-AC-6")]
fn request_member_and_header_agree_or_conflict() {
    let directory = tempfile::tempdir().unwrap();
    let program = format!("{HEADER}{FLOOR_HEADER}{BODY}");
    let agreeing = request(
        directory.path(),
        Some(&program),
        "native-compile/1",
        compile_body(Some(json!("floor"))),
    );
    command::compile(&agreeing).expect("an agreeing member compiles");

    let conflicting = request(
        directory.path(),
        None,
        "native-compile/1",
        compile_body(Some(json!("truncating"))),
    );
    let error = command::compile(&conflicting).expect_err("the laws conflict");
    let (code, cause, message) = refusal(&error, Operation::Package);
    assert_eq!(
        (code.as_str(), cause.as_str()),
        ("invalid_package", "conflicting-definition")
    );
    for named in ["floor", "d", "truncating", "integer_division"] {
        assert!(message.contains(named), "{message} names {named}");
    }
}

#[test]
#[trace("TC-2188", "FR-767-AC-3", "FSpec-TC-1652", "FSpec-FR-570-AC-6")]
fn malformed_or_unknown_member_refuses_before_any_file_is_read() {
    for (law, code, cause) in [
        (json!(null), "invalid-request", ""),
        (json!(3), "invalid-request", ""),
        (json!("ceiling"), "unknown_profile", "unsupported-selection"),
        (json!("Floor"), "unknown_profile", "unsupported-selection"),
    ] {
        let directory = tempfile::tempdir().unwrap();
        // No program file: a refusal that read it would be an io-error.
        let path = request(
            directory.path(),
            None,
            "native-compile/1",
            compile_body(Some(law.clone())),
        );
        let error = command::compile(&path).expect_err("the member refuses");
        let (actual_code, actual_cause, message) = refusal(&error, Operation::Package);
        assert_eq!(actual_code, code, "{law}");
        if !cause.is_empty() {
            assert_eq!(actual_cause, cause, "{law}");
        }
        assert!(message.contains("integer_division"), "{message}");
        if let Some(value) = law.as_str() {
            assert!(message.contains(value), "{message}");
        }
    }

    let directory = tempfile::tempdir().unwrap();
    let mut body = run_body(Some(json!("floor")), "q");
    body["package"] = json!({"file":"package.json"});
    let path = request(directory.path(), None, "native-run/1", body);
    let error = command::prepare_run(&path, command::RunLimits::default())
        .err()
        .expect("package and integer_division together refuse");
    let (code, _, message) = refusal(&error, Operation::Execute);
    assert_eq!(code, "invalid-request");
    assert!(message.contains("integer_division"), "{message}");
}

#[test]
#[trace("TC-2188", "FR-767-AC-4")]
fn no_member_and_no_header_still_refuses_missing_selection() {
    let directory = tempfile::tempdir().unwrap();
    let program = format!("{HEADER}{BODY}");
    let path = request(
        directory.path(),
        Some(&program),
        "native-compile/1",
        compile_body(None),
    );
    let error = command::compile(&path).expect_err("no law is chosen by default");
    let document = outcome(&error, Operation::Package);
    let diagnostics = document["diagnostics"].as_array().expect("diagnostics");
    assert_eq!(diagnostics.len(), 2, "one refusal per operator: {document}");
    for diagnostic in diagnostics {
        assert_eq!(diagnostic["code"], "missing_declaration", "{document}");
        assert_eq!(diagnostic["cause"], "missing-selection", "{document}");
    }

    let program = format!("{HEADER}{MOD_ONLY}");
    let path = request(
        directory.path(),
        Some(&program),
        "native-compile/1",
        compile_body(None),
    );
    command::compile(&path).expect("mod needs no law");
}

#[test]
#[trace("TC-2188", "FR-767-AC-5", "FSpec-TC-1652", "FSpec-FR-570-AC-6")]
fn library_entry_takes_its_own_law() {
    let directory = tempfile::tempdir().unwrap();
    write(
        directory.path(),
        "library.native",
        format!("{HEADER}{LIBRARY}").as_bytes(),
    );
    let program = format!("{HEADER}{IMPORTING}");
    let body = |library_law: Option<&str>, function: &str| {
        let mut body = run_body(Some(json!("floor")), function);
        let mut library =
            json!({"identity":"test/lib", "source":source("library.native", "test:lib")});
        if let Some(law) = library_law {
            library["integer_division"] = json!(law);
        }
        body["libraries"] = json!([library]);
        body
    };
    let path = request(
        directory.path(),
        Some(&program),
        "native-run/1",
        body(Some("truncating"), "viaLib"),
    );
    assert_eq!(returned(&path), "-2");
    let path = request(
        directory.path(),
        None,
        "native-run/1",
        body(Some("truncating"), "q"),
    );
    assert_eq!(returned(&path), "-3");

    let path = request(directory.path(), None, "native-run/1", body(None, "q"));
    let error = command::run(&path).expect_err("the library has no law");
    let (code, cause, _) = refusal(&error, Operation::Execute);
    assert_eq!(
        (code.as_str(), cause.as_str()),
        ("missing_declaration", "missing-selection")
    );
}

#[test]
#[trace("TC-2188", "FR-767-AC-6")]
fn member_over_a_unit_without_div_or_rem_leaves_the_package_unchanged() {
    let directory = tempfile::tempdir().unwrap();
    let program = format!("{HEADER}{MOD_ONLY}");
    let without = request(
        directory.path(),
        Some(&program),
        "native-compile/1",
        compile_body(None),
    );
    let without = command::compile(&without).expect("compiles without a law");
    let with = request(
        directory.path(),
        None,
        "native-compile/1",
        compile_body(Some(json!("floor"))),
    );
    let with = command::compile(&with).expect("compiles with an unused law");
    assert_eq!(with, without);
}

#[test]
#[trace("TC-2196", "FR-767-AC-8")]
fn top_level_law_reaches_every_unit_of_a_multi_unit_program() {
    let directory = tempfile::tempdir().unwrap();
    let units = [
        ("unit-a.native", "test:unit-a", "qa"),
        ("unit-b.native", "test:unit-b", "qb"),
    ];
    let mut program = Vec::new();
    for (file, identity, function) in units {
        let text = format!(
            "{HEADER}function {function} using v(a: Int[-7, 7], b: Int[1, 3]): Int[-7, 7] pure {{ a div b }}\n"
        );
        write(directory.path(), file, text.as_bytes());
        program.push(source(file, identity));
    }
    let body = |law: Option<&str>| {
        let mut body = json!({"models":[], "program": program.clone()});
        if let Some(law) = law {
            body["integer_division"] = json!(law);
        }
        body
    };
    let path = request(
        directory.path(),
        None,
        "native-compile/1",
        body(Some("floor")),
    );
    let bytes = command::compile(&path).expect("every unit compiles under floor");
    assert_eq!(division_laws(&bytes), vec![law_identity("floor"); 2]);

    let path = request(directory.path(), None, "native-compile/1", body(None));
    let error = command::compile(&path).expect_err("no unit has a law");
    let document = outcome(&error, Operation::Package);
    assert_eq!(
        document["diagnostics"][0]["cause"], "missing-selection",
        "{document}"
    );
}
