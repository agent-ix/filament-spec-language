// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-495: short-circuit guards retain definedness facts for integer division.

use std::path::Path;

use fsl_command::command;
use ix_trace_rs::trace;
use serde_json::json;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
                    profile v = \"filament.value.complete/v1\";\n\
                    profile d = \"filament.value.integer-division.truncating/v1\";\n";

fn write_run(
    directory: &Path,
    program: &str,
    function: &str,
    x: i64,
    y: i64,
) -> std::path::PathBuf {
    std::fs::write(directory.join("program.native"), program).expect("write program source");
    let request = json!({"format":"native-run/1", "request":{
        "models":[],
        "program":{"source":{
            "file":"program.native", "authority":"example",
            "identity":"test:short-circuit", "document":"ShortCircuit", "formal_revision":1
        }},
        "call":{"function":function, "arguments":[
            {"parameter":"x", "value":x}, {"parameter":"y", "value":y}
        ]}
    }});
    let path = directory.join("run.json");
    std::fs::write(&path, serde_json::to_vec(&request).expect("encode request"))
        .expect("write request");
    path
}

fn assert_boolean(directory: &Path, program: &str, function: &str, x: i64, y: i64, expected: bool) {
    let path = write_run(directory, program, function, x, y);
    let result = command::run(&path, &crate::negotiation::Fr290)
        .unwrap_or_else(|error| panic!("{function}({x}, {y}) runs: {error}"));
    assert_eq!(result.exit_code, 0, "{function}({x}, {y}) completes");
    let document = result.value.as_value();
    assert_eq!(
        document["outcome"],
        json!({"kind":"completed", "value":{"boolean":expected}}),
        "{function}({x}, {y})"
    );
}

/// FR-495 short-circuit guards: each logical form must preserve the same
/// nonzero fact for div, rem and mod, and skip the operator when `y` is zero.
#[trace("FR-495-AC-3", "FR-495-AC-17", "FR-495-AC-18", "TC-1284", "TC-1783")]
#[test]
fn guarded_integer_division_in_and_or_and_implies_compiles_and_runs() {
    let directory = tempfile::tempdir().expect("temporary run directory");
    for (operator, expected) in [("div", 3), ("rem", 1), ("mod", 1)] {
        for (connective, guarded, zero_value) in [
            ("and", "y != 0 and", false),
            ("or", "y = 0 or", true),
            ("implies", "y != 0 implies", true),
        ] {
            let function = format!("{operator}_{connective}");
            let program = format!(
                "{HEADER}function {function} using v(x: Int[0, 9], y: Int[0, 9]): Boolean pure {{ {guarded} x {operator} y = {expected} }}\n"
            );
            assert_boolean(directory.path(), &program, &function, 7, 0, zero_value);
            assert_boolean(directory.path(), &program, &function, 7, 2, true);
        }
    }
}
