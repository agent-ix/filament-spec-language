// SPDX-License-Identifier: AGPL-3.0-or-later
//! Drive checked-in source programs through the public command entry points.

use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const AREAS: &[&str] = &[
    "values",
    "collections",
    "operations",
    "temporal",
    "protocol",
    "refinement",
    "probability",
];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Expectation {
    verb: String,
    exit_code: u8,
    category: Option<String>,
    last_stage: Option<String>,
    code: Option<String>,
    cause: Option<String>,
    result: Option<Value>,
    flips_with: Option<String>,
}

#[derive(Debug)]
struct Actual {
    exit_code: u8,
    category: Option<String>,
    last_stage: Option<String>,
    code: Option<String>,
    cause: Option<String>,
    result: Option<Value>,
}

/// The corpus starts with no area directories; later area tickets add each
/// directory. A created directory must contain at least one `.native` file.
fn run_area(area: &str) {
    let root = corpus_root();
    let directory = root.join(area);
    if !directory.exists() {
        return;
    }
    let report = run_directory(&directory).unwrap_or_else(|error| panic!("{error}"));
    println!("{report}");
}

fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus")
}

fn run_directory(directory: &Path) -> Result<String, String> {
    let entries = std::fs::read_dir(directory)
        .map_err(|error| format!("{}: cannot read corpus area: {error}", directory.display()))?;
    let mut programs = BTreeSet::new();
    let mut expectations = BTreeSet::new();
    for entry in entries {
        let entry = entry.map_err(|error| {
            format!("{}: cannot read corpus entry: {error}", directory.display())
        })?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if let Some(stem) = name.strip_suffix(".native") {
            programs.insert(stem.to_owned());
        } else if let Some(stem) = name.strip_suffix(".expected.json") {
            expectations.insert(stem.to_owned());
        }
    }

    if let Some(orphan) = programs.difference(&expectations).next() {
        return Err(format!(
            "{}: program has no expectation file",
            directory.join(format!("{orphan}.native")).display()
        ));
    }
    if let Some(orphan) = expectations.difference(&programs).next() {
        return Err(format!(
            "{}: expectation has no program",
            directory.join(format!("{orphan}.expected.json")).display()
        ));
    }
    if programs.is_empty() {
        return Err(format!(
            "{}: corpus area has no .native programs",
            directory.display()
        ));
    }

    let mut flips = BTreeMap::<String, usize>::new();
    let mut failures = Vec::new();
    for stem in programs {
        let program_path = directory.join(format!("{stem}.native"));
        let expectation_path = directory.join(format!("{stem}.expected.json"));
        let expected_bytes = match std::fs::read(&expectation_path) {
            Ok(bytes) => bytes,
            Err(error) => {
                failures.push(format!("{}: {error}", expectation_path.display()));
                continue;
            }
        };
        let expected: Expectation = match serde_json::from_slice(&expected_bytes) {
            Ok(expected) => expected,
            Err(error) => {
                failures.push(format!(
                    "{}: invalid expectation: {error}",
                    expectation_path.display()
                ));
                continue;
            }
        };
        if let Some(key) = &expected.flips_with {
            *flips.entry(key.clone()).or_default() += 1;
        }
        let actual = match execute(&program_path, &expected) {
            Ok(actual) => actual,
            Err(error) => {
                failures.push(error);
                continue;
            }
        };
        let mismatch = mismatch(&program_path, &expected, &actual);
        if let Some(mismatch) = mismatch {
            failures.push(mismatch);
        }
    }

    let flip_report = if flips.is_empty() {
        "flips_with: none".to_owned()
    } else {
        format!(
            "flips_with: {}",
            flips
                .iter()
                .map(|(key, count)| format!("{key} ({count})"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    if failures.is_empty() {
        Ok(flip_report)
    } else {
        failures.push(flip_report);
        Err(failures.join("\n"))
    }
}

fn execute(program: &Path, expected: &Expectation) -> Result<Actual, String> {
    let request = program.with_extension("request.json");
    if request.exists() {
        execute_request(&request, expected)
    } else {
        execute_compile(program, expected)
    }
}

fn execute_request(request: &Path, expected: &Expectation) -> Result<Actual, String> {
    let request_json: Value = serde_json::from_slice(
        &std::fs::read(request).map_err(|error| format!("{}: {error}", request.display()))?,
    )
    .map_err(|error| format!("{}: invalid request: {error}", request.display()))?;
    match request_json.get("format").and_then(Value::as_str) {
        Some("native-run/1") => {
            if expected.verb != "run" {
                return Err(format!(
                    "{}: expected verb {:?} does not match native-run/1 request",
                    request.display(),
                    expected.verb
                ));
            }
            match fsl_command::command::run(request) {
                Ok(result) => {
                    let value = result.value.as_value().clone();
                    Ok(Actual {
                        exit_code: result.exit_code,
                        category: None,
                        last_stage: None,
                        code: value.get("code").and_then(string_member).map(str::to_owned),
                        cause: value
                            .get("cause")
                            .and_then(string_member)
                            .map(str::to_owned),
                        result: value
                            .get("outcome")
                            .and_then(|outcome| outcome.get("value"))
                            .cloned(),
                    })
                }
                Err(error) => {
                    let value = error.value().map_err(|error| {
                        format!("{}: cannot encode refusal: {error}", request.display())
                    })?;
                    Ok(actual_error(
                        error.category().as_str(),
                        error.category().exit_code(),
                        &value,
                    ))
                }
            }
        }
        Some("native-compile/1") => {
            if expected.verb != "compile" {
                return Err(format!(
                    "{}: expected verb {:?} does not match native-compile/1 request",
                    request.display(),
                    expected.verb
                ));
            }
            match fsl_command::command::compile(request) {
                Ok(_) => Ok(Actual {
                    exit_code: 0,
                    category: Some("success".to_owned()),
                    last_stage: None,
                    code: None,
                    cause: None,
                    result: None,
                }),
                Err(error) => {
                    let value = error.artifact_value().map_err(|error| {
                        format!("{}: cannot encode refusal: {error}", request.display())
                    })?;
                    Ok(actual_error(
                        error.category().as_str(),
                        error.category().exit_code(),
                        &value,
                    ))
                }
            }
        }
        _ => Err(format!(
            "{}: request format must be native-run/1 or native-compile/1",
            request.display()
        )),
    }
}

/// With no request sidecar, `compile` is the current public API's closest
/// check entry point. `verb: check` remains expectation data; no new command
/// verb or result shape is introduced here.
fn execute_compile(program: &Path, expected: &Expectation) -> Result<Actual, String> {
    if expected.verb != "check" {
        return Err(format!(
            "{}: no request is present, so expected verb must be check",
            program.display()
        ));
    }
    let scratch = tempfile::tempdir().map_err(|error| {
        format!(
            "{}: cannot create compile request directory: {error}",
            program.display()
        )
    })?;
    std::fs::copy(program, scratch.path().join("program.native"))
        .map_err(|error| format!("{}: cannot stage source: {error}", program.display()))?;
    let request = json!({
        "format": "native-compile/1",
        "request": {
            "models": [],
            "program": {"source": {
                "file": "program.native",
                "authority": "agent-ix",
                "identity": format!("test:corpus:{}", program.file_stem().and_then(|s| s.to_str()).unwrap_or("program")),
                "document": program.file_name().and_then(|s| s.to_str()).unwrap_or("program.native"),
                "formal_revision": 1
            }}
        }
    });
    let request_path = scratch.path().join("request.json");
    std::fs::write(
        &request_path,
        serde_json::to_vec(&request).map_err(|error| error.to_string())?,
    )
    .map_err(|error| {
        format!(
            "{}: cannot write compile request: {error}",
            program.display()
        )
    })?;
    match fsl_command::command::compile(&request_path) {
        Ok(_) => Ok(Actual {
            exit_code: 0,
            category: Some("success".to_owned()),
            last_stage: None,
            code: None,
            cause: None,
            result: None,
        }),
        Err(error) => {
            let value = error.artifact_value().map_err(|error| {
                format!("{}: cannot encode refusal: {error}", program.display())
            })?;
            Ok(actual_error(
                error.category().as_str(),
                error.category().exit_code(),
                &value,
            ))
        }
    }
}

fn string_member(value: &Value) -> Option<&str> {
    value.as_str()
}

fn actual_error(category: &str, exit_code: u8, value: &Value) -> Actual {
    Actual {
        exit_code,
        category: Some(category.to_owned()),
        last_stage: value
            .get("stage")
            .and_then(string_member)
            .map(str::to_owned),
        code: value.get("code").and_then(string_member).map(str::to_owned),
        cause: value
            .get("cause")
            .and_then(string_member)
            .map(str::to_owned),
        result: None,
    }
}

fn mismatch(program: &Path, expected: &Expectation, actual: &Actual) -> Option<String> {
    let differs = expected.exit_code != actual.exit_code
        || exposed_field_differs(&expected.category, &actual.category)
        || exposed_field_differs(&expected.last_stage, &actual.last_stage)
        || optional_differs(&expected.code, &actual.code)
        || optional_differs(&expected.cause, &actual.cause)
        || expected
            .result
            .as_ref()
            .is_some_and(|value| actual.result.as_ref() != Some(value));
    differs.then(|| {
        format!(
            "{}: expectation mismatch: verb={} exit_code expected={} actual={} category expected={} actual={} last_stage expected={} actual={} code expected={} actual={} cause expected={} actual={} flips_with={}",
            program.display(),
            expected.verb,
            expected.exit_code,
            actual.exit_code,
            display_option(&expected.category),
            display_option(&actual.category),
            display_option(&expected.last_stage),
            display_option(&actual.last_stage),
            display_option(&expected.code),
            display_option(&actual.code),
            display_option(&expected.cause),
            display_option(&actual.cause),
            expected.flips_with.as_deref().unwrap_or("none"),
        )
    })
}

fn exposed_field_differs(expected: &Option<String>, actual: &Option<String>) -> bool {
    expected
        .as_ref()
        .zip(actual.as_ref())
        .is_some_and(|(expected, actual)| expected != actual)
}

fn optional_differs(expected: &Option<String>, actual: &Option<String>) -> bool {
    expected
        .as_ref()
        .is_some_and(|expected| actual.as_ref() != Some(expected))
}

fn display_option(value: &Option<String>) -> &str {
    value.as_deref().unwrap_or("<not exposed>")
}

#[test]
fn corpus_values() {
    run_area("values");
}

#[test]
fn corpus_collections() {
    run_area("collections");
}

#[test]
fn corpus_operations() {
    run_area("operations");
}

#[test]
fn corpus_temporal() {
    run_area("temporal");
}

#[test]
fn corpus_protocol() {
    run_area("protocol");
}

#[test]
fn corpus_refinement() {
    run_area("refinement");
}

#[test]
fn corpus_probability() {
    run_area("probability");
}

const ACCEPTING_PROGRAM: &str = include_str!("../fixtures/spine-compile.native");

fn expectation(verb: &str, exit_code: u8) -> String {
    json!({"verb":verb,"exit_code":exit_code,"category":"success"}).to_string()
}

fn write_case(directory: &Path, name: &str, program: &str, expected: &str) {
    std::fs::write(directory.join(format!("{name}.native")), program).unwrap();
    std::fs::write(directory.join(format!("{name}.expected.json")), expected).unwrap();
}

#[test]
fn corpus_runner_accepts_matching_expectation() {
    let directory = tempfile::tempdir().unwrap();
    write_case(
        directory.path(),
        "accepting",
        ACCEPTING_PROGRAM,
        &expectation("check", 0),
    );
    assert_eq!(run_directory(directory.path()).unwrap(), "flips_with: none");
}

#[test]
fn corpus_runner_detects_exit_code_mismatch() {
    let directory = tempfile::tempdir().unwrap();
    write_case(
        directory.path(),
        "accepting",
        ACCEPTING_PROGRAM,
        &expectation("check", 10),
    );
    let error = run_directory(directory.path()).unwrap_err();
    assert!(error.contains("accepting.native"), "{error}");
    assert!(error.contains("exit_code expected=10 actual=0"), "{error}");
}

#[test]
fn corpus_runner_detects_code_and_cause_mismatch() {
    let directory = tempfile::tempdir().unwrap();
    let invalid = "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.model.complete/v1\";\n";
    write_case(
            directory.path(),
            "refusing",
            invalid,
            &json!({"verb":"check","exit_code":20,"category":"refusal","code":"wrong_code","cause":"wrong_cause"}).to_string(),
        );
    let error = run_directory(directory.path()).unwrap_err();
    assert!(
        error.contains("code expected=wrong_code actual="),
        "{error}"
    );
    assert!(
        error.contains("cause expected=wrong_cause actual="),
        "{error}"
    );
}

#[test]
fn corpus_runner_rejects_program_without_expectation() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("orphan.native"), ACCEPTING_PROGRAM).unwrap();
    let error = run_directory(directory.path()).unwrap_err();
    assert!(error.contains("orphan.native"), "{error}");
    assert!(error.contains("no expectation file"), "{error}");
}

#[test]
fn corpus_runner_rejects_expectation_without_program() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(
        directory.path().join("orphan.expected.json"),
        expectation("check", 0),
    )
    .unwrap();
    let error = run_directory(directory.path()).unwrap_err();
    assert!(error.contains("orphan.expected.json"), "{error}");
    assert!(error.contains("no program"), "{error}");
}

#[test]
fn corpus_runner_reports_flip_keys() {
    let directory = tempfile::tempdir().unwrap();
    let expected =
        json!({"verb":"check","exit_code":0,"category":"success","flips_with":"QSL-1965"})
            .to_string();
    write_case(directory.path(), "one", ACCEPTING_PROGRAM, &expected);
    write_case(directory.path(), "two", ACCEPTING_PROGRAM, &expected);
    assert_eq!(
        run_directory(directory.path()).unwrap(),
        "flips_with: QSL-1965 (2)"
    );
}

#[test]
fn corpus_runner_executes_all_programs_before_reporting_mismatches() {
    let directory = tempfile::tempdir().unwrap();
    let expected = json!({"verb":"check","exit_code":10,"flips_with":"QSL-1965"}).to_string();
    write_case(directory.path(), "one", ACCEPTING_PROGRAM, &expected);
    write_case(directory.path(), "two", ACCEPTING_PROGRAM, &expected);

    let error = run_directory(directory.path()).unwrap_err();
    assert!(error.contains("one.native"), "{error}");
    assert!(error.contains("two.native"), "{error}");
    assert!(error.contains("flips_with: QSL-1965 (2)"), "{error}");
}

#[test]
fn corpus_runner_uses_public_entry_point() {
    let source = include_str!("source_corpus.rs");
    for forbidden in ["use fsl_package", "use fsl_semantics", "use fsl_eval"] {
        assert!(
            !source
                .lines()
                .any(|line| line.trim_start().starts_with(forbidden)),
            "found {forbidden}"
        );
    }
    assert!(source.contains("fsl_command::command::compile"));
    assert!(source.contains("fsl_command::command::run"));
}

#[test]
fn absent_area_is_allowed_but_created_empty_area_is_rejected() {
    assert!(!AREAS.is_empty());
    let directory = tempfile::tempdir().unwrap();
    let error = run_directory(directory.path()).unwrap_err();
    assert!(error.contains("no .native programs"), "{error}");
}
