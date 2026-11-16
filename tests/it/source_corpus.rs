// SPDX-License-Identifier: AGPL-3.0-or-later
//! Drive checked-in source programs through the public command entry points.

use filament_exact::Cancel;
use fsl_replay::{Operation, OutcomeDocument};
use ix_trace_rs::trace;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const AREA_CATALOG: &str = include_str!("../corpus/areas.txt");

fn corpus_areas() -> impl Iterator<Item = &'static str> {
    AREA_CATALOG
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
}

/// A member that may be absent, `null` or a string.
fn member<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Option<String>>, D::Error> {
    Option::<String>::deserialize(deserializer).map(Some)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Expectation {
    verb: String,
    exit_code: u8,
    category: Option<String>,
    /// Absent: not checked. `null`: the outcome reached no stage.
    #[serde(default, deserialize_with = "member")]
    last_stage: Option<Option<String>>,
    code: Option<String>,
    cause: Option<String>,
    result: Option<Value>,
    flips_with: Option<String>,
}

#[derive(Debug)]
struct Actual {
    exit_code: u8,
    category: Option<String>,
    last_stage: Option<Option<String>>,
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
    let mut requests = BTreeSet::new();
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
        } else if let Some(stem) = name.strip_suffix(".request.json") {
            requests.insert(stem.to_owned());
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
    if let Some(orphan) = requests.difference(&programs).next() {
        return Err(format!(
            "{}: request has no paired program",
            directory.join(format!("{orphan}.request.json")).display()
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
        execute_request(&request, program, expected)
    } else {
        execute_compile(program, expected)
    }
}

fn execute_request(
    request: &Path,
    paired_program: &Path,
    expected: &Expectation,
) -> Result<Actual, String> {
    let request_json: Value = serde_json::from_slice(
        &std::fs::read(request).map_err(|error| format!("{}: {error}", request.display()))?,
    )
    .map_err(|error| format!("{}: invalid request: {error}", request.display()))?;
    require_paired_program(request, paired_program, &request_json)?;
    match request_json.get("format").and_then(Value::as_str) {
        Some(format @ ("native-run/1" | "native-monitor/1" | "native-analyze/1")) => {
            let (verb, operation) = match format {
                "native-run/1" => ("run", Operation::Execute),
                "native-monitor/1" => ("monitor", Operation::Monitor),
                _ => ("analyze", Operation::Analyze),
            };
            if expected.verb != verb {
                return Err(format!(
                    "{}: expected verb {:?} does not match {format} request",
                    request.display(),
                    expected.verb
                ));
            }
            let ran = if format == "native-analyze/1" {
                fsl_command::command::analyze(request, &OneCandidate, &Cancel::new())
            } else {
                fsl_command::command::run(request)
            };
            match ran {
                Ok(result) => {
                    let value = result.value.as_value().clone();
                    let outcome = value.get("outcome");
                    let (category, last_stage) = match &result.outcome {
                        Some(document) => lifecycle(document)?,
                        None => lifecycle_members(&value)?,
                    };
                    let (category, last_stage) = (Some(category), Some(last_stage));
                    Ok(Actual {
                        exit_code: result.exit_code,
                        category,
                        last_stage,
                        code: value
                            .get("code")
                            .or_else(|| outcome.and_then(|outcome| outcome.get("code")))
                            .and_then(string_member)
                            .map(str::to_owned),
                        cause: value
                            .get("cause")
                            .or_else(|| outcome.and_then(|outcome| outcome.get("cause")))
                            .and_then(string_member)
                            .map(str::to_owned),
                        result: outcome.and_then(|outcome| outcome.get("value")).cloned(),
                    })
                }
                Err(error) => {
                    let value = error.value().map_err(|error| {
                        format!("{}: cannot encode refusal: {error}", request.display())
                    })?;
                    actual_error(
                        &error.outcome_document(operation),
                        error.category().exit_code(),
                        &value,
                    )
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
            match fsl_command::command::compile_outcome(request) {
                Ok(compiled) => {
                    let (category, last_stage) = lifecycle(compiled.document())?;
                    Ok(Actual {
                        exit_code: 0,
                        category: Some(category),
                        last_stage: Some(last_stage),
                        code: None,
                        cause: None,
                        result: None,
                    })
                }
                Err(error) => {
                    let value = error.artifact_value().map_err(|error| {
                        format!("{}: cannot encode refusal: {error}", request.display())
                    })?;
                    Ok(actual_error(
                        &error.outcome_document(Operation::Package),
                        error.category().exit_code(),
                        &value,
                    )?)
                }
            }
        }
        _ => Err(format!(
            "{}: request format must be native-run/1 or native-compile/1",
            request.display()
        )),
    }
}

fn require_paired_program(
    request: &Path,
    paired_program: &Path,
    document: &Value,
) -> Result<(), String> {
    let program = document
        .get("request")
        .and_then(|request| request.get("program"))
        .ok_or_else(|| format!("{}: request has no program selection", request.display()))?;
    let sources = if let Some(source) = program.get("source") {
        vec![source]
    } else if let Some(sources) = program.as_array() {
        sources.iter().collect()
    } else {
        return Err(format!(
            "{}: request program must select a source object or array",
            request.display()
        ));
    };
    let paired_program = paired_program.canonicalize().map_err(|error| {
        format!(
            "{}: cannot resolve paired program: {error}",
            paired_program.display()
        )
    })?;
    let request_directory = request
        .parent()
        .ok_or_else(|| format!("{}: request has no parent directory", request.display()))?;
    let mut selected = Vec::new();
    for source in sources {
        let Some(file) = source.get("file").and_then(Value::as_str) else {
            continue;
        };
        let selected_path = request_directory.join(file);
        if selected_path.canonicalize().ok().as_ref() == Some(&paired_program) {
            return Ok(());
        }
        selected.push(file.to_owned());
    }
    Err(format!(
        "{}: request program selection {:?} does not point to paired corpus program {}",
        request.display(),
        selected,
        paired_program.display()
    ))
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
    match fsl_command::command::compile_outcome(&request_path) {
        Ok(compiled) => {
            let (category, last_stage) = lifecycle(compiled.document())?;
            Ok(Actual {
                exit_code: 0,
                category: Some(category),
                last_stage: Some(last_stage),
                code: None,
                cause: None,
                result: None,
            })
        }
        Err(error) => {
            let value = error.artifact_value().map_err(|error| {
                format!("{}: cannot encode refusal: {error}", program.display())
            })?;
            Ok(actual_error(
                &error.outcome_document(Operation::Package),
                error.category().exit_code(),
                &value,
            )?)
        }
    }
}

fn string_member(value: &Value) -> Option<&str> {
    value.as_str()
}

fn actual_error(
    document: &OutcomeDocument,
    exit_code: u8,
    value: &Value,
) -> Result<Actual, String> {
    let (category, last_stage) = lifecycle(document)?;
    Ok(Actual {
        exit_code,
        category: Some(category),
        last_stage: Some(last_stage),
        code: value.get("code").and_then(string_member).map(str::to_owned),
        cause: value
            .get("cause")
            .and_then(string_member)
            .map(str::to_owned),
        result: None,
    })
}

/// The `category` and `last_stage` members of a lifecycle outcome document.
fn lifecycle(document: &OutcomeDocument) -> Result<(String, Option<String>), String> {
    let bytes = document
        .to_bytes()
        .map_err(|error| format!("cannot encode the lifecycle outcome: {error}"))?;
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid lifecycle outcome: {error}"))?;
    lifecycle_members(&value)
}

/// The `category` and `last_stage` members of a `filament-outcome/1` document.
fn lifecycle_members(value: &Value) -> Result<(String, Option<String>), String> {
    let object = value
        .as_object()
        .ok_or_else(|| "the lifecycle outcome is not an object".to_owned())?;
    let category = object
        .get("category")
        .and_then(Value::as_str)
        .ok_or_else(|| "the lifecycle outcome has no category member".to_owned())?;
    let last_stage = match object.get("last_stage") {
        Some(Value::Null) => None,
        Some(Value::String(stage)) => Some(stage.clone()),
        _ => return Err("the lifecycle outcome has no last_stage member".to_owned()),
    };
    Ok((category.to_owned(), last_stage))
}

fn mismatch(program: &Path, expected: &Expectation, actual: &Actual) -> Option<String> {
    let differs = expected.exit_code != actual.exit_code
        || required_differs(&expected.category, &actual.category)
        || required_differs(&expected.last_stage, &actual.last_stage)
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
            display_stage(&expected.last_stage),
            display_stage(&actual.last_stage),
            display_option(&expected.code),
            display_option(&actual.code),
            display_option(&expected.cause),
            display_option(&actual.cause),
            expected.flips_with.as_deref().unwrap_or("none"),
        )
    })
}

/// An expected member must be present and equal in the actual outcome; an
/// outcome that does not expose it differs.
fn required_differs<T: PartialEq>(expected: &Option<T>, actual: &Option<T>) -> bool {
    expected
        .as_ref()
        .is_some_and(|expected| actual.as_ref() != Some(expected))
}

fn optional_differs(expected: &Option<String>, actual: &Option<String>) -> bool {
    expected
        .as_ref()
        .is_some_and(|expected| actual.as_ref() != Some(expected))
}

fn display_stage(value: &Option<Option<String>>) -> &str {
    match value {
        None => "<not exposed>",
        Some(None) => "null",
        Some(Some(stage)) => stage,
    }
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
const RUN_PROGRAM: &str = include_str!("../fixtures/spine-run.native");

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
fn corpus_runner_reads_code_from_run_outcome() {
    let directory = tempfile::tempdir().unwrap();
    write_case(
        directory.path(),
        "run-refusal",
        RUN_PROGRAM,
        &json!({
            "verb":"run",
            "exit_code":20,
            "code":"wrong_code",
            "cause":"wrong_cause"
        })
        .to_string(),
    );
    let request = json!({
        "format":"native-run/1",
        "request":{
            "models":[],
            "program":{"source":{
                "file":"run-refusal.native",
                "authority":"agent-ix",
                "identity":"test:corpus:run-refusal",
                "document":"RunRefusal",
                "formal_revision":1
            }},
            "call":{"function":"nope","arguments":[]}
        }
    });
    std::fs::write(
        directory.path().join("run-refusal.request.json"),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();

    let error = run_directory(directory.path()).unwrap_err();
    assert!(
        error.contains("code expected=wrong_code actual=missing_declaration"),
        "{error}"
    );
}

#[test]
fn corpus_runner_rejects_request_for_a_different_program() {
    let directory = tempfile::tempdir().unwrap();
    write_case(
        directory.path(),
        "paired",
        ACCEPTING_PROGRAM,
        &expectation("run", 0),
    );
    let request = json!({
        "format":"native-run/1",
        "request":{
            "models":[],
            "program":{"source":{
                "file":"different.native",
                "authority":"agent-ix",
                "identity":"test:corpus:different",
                "document":"Different",
                "formal_revision":1
            }},
            "call":{"function":"seven","arguments":[]}
        }
    });
    std::fs::write(
        directory.path().join("paired.request.json"),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();

    let error = run_directory(directory.path()).unwrap_err();
    assert!(error.contains("different.native"), "{error}");
    assert!(
        error.contains("does not point to paired corpus program"),
        "{error}"
    );
}

#[test]
fn corpus_runner_rejects_request_without_a_paired_program() {
    let directory = tempfile::tempdir().unwrap();
    let request = json!({
        "format":"native-run/1",
        "request":{
            "models":[],
            "program":{"source":{
                "file":"missing.native",
                "authority":"agent-ix",
                "identity":"test:corpus:missing",
                "document":"Missing",
                "formal_revision":1
            }},
            "call":{"function":"seven","arguments":[]}
        }
    });
    std::fs::write(
        directory.path().join("orphan.request.json"),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();

    let error = run_directory(directory.path()).unwrap_err();
    assert!(error.contains("orphan.request.json"), "{error}");
    assert!(error.contains("request has no paired program"), "{error}");
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
    use syn::visit::Visit;

    #[derive(Default)]
    struct Calls {
        compile: bool,
        run: bool,
        forbidden: BTreeSet<String>,
    }

    impl<'ast> Visit<'ast> for Calls {
        fn visit_path(&mut self, path: &'ast syn::Path) {
            if let Some(segment) = path.segments.first() {
                if matches!(
                    segment.ident.to_string().as_str(),
                    "fsl_package" | "fsl_semantics" | "fsl_eval"
                ) {
                    self.forbidden.insert(segment.ident.to_string());
                }
            }
            syn::visit::visit_path(self, path);
        }

        fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
            if let syn::Expr::Path(path) = call.func.as_ref() {
                let segments = path
                    .path
                    .segments
                    .iter()
                    .map(|segment| segment.ident.to_string())
                    .collect::<Vec<_>>();
                if segments
                    == [
                        "fsl_command".to_owned(),
                        "command".to_owned(),
                        "compile_outcome".to_owned(),
                    ]
                {
                    self.compile = true;
                }
                if segments
                    == [
                        "fsl_command".to_owned(),
                        "command".to_owned(),
                        "run".to_owned(),
                    ]
                {
                    self.run = true;
                }
            }
            syn::visit::visit_expr_call(self, call);
        }
    }

    let source = include_str!("source_corpus.rs");
    let syntax = syn::parse_file(source).expect("source_corpus.rs must parse as Rust");
    let mut calls = Calls::default();
    calls.visit_file(&syntax);
    assert!(
        calls.forbidden.is_empty(),
        "source corpus code references forbidden crates: {:?}",
        calls.forbidden
    );
    assert!(
        calls.compile,
        "runner must call fsl_command::command::compile_outcome"
    );
    assert!(calls.run, "runner must call fsl_command::command::run");
}

#[test]
fn absent_area_is_allowed_but_created_empty_area_is_rejected() {
    assert!(corpus_areas().next().is_some());
    let directory = tempfile::tempdir().unwrap();
    let error = run_directory(directory.path()).unwrap_err();
    assert!(error.contains("no .native programs"), "{error}");
}

fn write_run_request(directory: &Path, stem: &str, function: &str) {
    let request = json!({
        "format":"native-run/1",
        "request":{
            "models":[],
            "program":{"source":{
                "file": format!("{stem}.native"),
                "authority":"agent-ix",
                "identity": format!("test:corpus:{stem}"),
                "document":"Run",
                "formal_revision":1
            }},
            "call":{"function":function,"arguments":[]}
        }
    });
    std::fs::write(
        directory.join(format!("{stem}.request.json")),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();
}

/// FR-286-AC-1, TC-770: the runner compares the lifecycle outcome's
/// `category` and `last_stage` members, a `null` stage included.
#[test]
#[trace("TC-770", "FR-286-AC-1", "FR-286-AC-5", "FR-286-AC-6")]
fn corpus_runner_compares_category_and_last_stage() {
    // A compiled package reached S4.
    let directory = tempfile::tempdir().unwrap();
    write_case(
        directory.path(),
        "accepting",
        ACCEPTING_PROGRAM,
        &json!({"verb":"check","exit_code":0,"category":"success","last_stage":"S4"}).to_string(),
    );
    assert_eq!(run_directory(directory.path()).unwrap(), "flips_with: none");

    // A refused call stops at S6a.
    let directory = tempfile::tempdir().unwrap();
    write_case(
        directory.path(),
        "refusal",
        RUN_PROGRAM,
        &json!({"verb":"run","exit_code":20,"category":"refusal","last_stage":"S6a"}).to_string(),
    );
    write_run_request(directory.path(), "refusal", "nope");
    assert_eq!(run_directory(directory.path()).unwrap(), "flips_with: none");

    // A wrong stage, a `null` stage and a wrong category are mismatches.
    for (member, wrong) in [
        ("last_stage", json!("S3")),
        ("last_stage", Value::Null),
        ("category", json!("violation")),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let mut expected =
            json!({"verb":"check","exit_code":0,"category":"success","last_stage":"S4"});
        expected[member] = wrong;
        write_case(
            directory.path(),
            "accepting",
            ACCEPTING_PROGRAM,
            &expected.to_string(),
        );
        let error = run_directory(directory.path()).unwrap_err();
        assert!(error.contains(&format!("{member} expected=")), "{error}");
    }
}

/// FR-286-AC-1, TC-770: an outcome that omits `category` or `last_stage` is
/// a mismatch against an expectation naming it, never a silent pass.
#[test]
#[trace("TC-770", "FR-286-AC-1")]
fn corpus_runner_detects_missing_lifecycle_fields() {
    let expected: Expectation = serde_json::from_value(
        json!({"verb":"run","exit_code":0,"category":"success","last_stage":null}),
    )
    .unwrap();
    assert_eq!(expected.last_stage, Some(None));
    let omitted: Expectation = serde_json::from_value(json!({"verb":"run","exit_code":0})).unwrap();
    assert_eq!(omitted.last_stage, None);

    let actual = Actual {
        exit_code: 0,
        category: None,
        last_stage: None,
        code: None,
        cause: None,
        result: None,
    };
    let mismatch =
        mismatch_of(&expected, &actual).expect("members the outcome does not carry are a mismatch");
    assert!(mismatch.contains("category expected=success actual=<not exposed>"));
    assert!(mismatch.contains("last_stage expected=null actual=<not exposed>"));

    let complete = Actual {
        category: Some("success".to_owned()),
        last_stage: Some(None),
        ..actual
    };
    assert_eq!(mismatch_of(&expected, &complete), None);
}

fn mismatch_of(expected: &Expectation, actual: &Actual) -> Option<String> {
    mismatch(Path::new("case.native"), expected, actual)
}

/// FSpec FR-290's negotiation for the analyze corpus cases: one candidate is
/// `supported`, none `unsupported`.
struct OneCandidate;

impl fsl_analyze::analyze::document::Negotiate for OneCandidate {
    fn negotiate(
        &self,
        item: &fsl_analyze::analyze::ClaimItem,
        candidates: &fsl_route::CandidateOutcome,
    ) -> (
        fsl_route::routing::Disposition,
        Option<fsl_analyze::analyze::Diagnostic>,
    ) {
        use fsl_analyze::analyze::{Diagnostic, Payload};
        use fsl_route::routing::Disposition;
        match candidates {
            fsl_route::CandidateOutcome::Candidates(set) if set.candidates().len() == 1 => {
                (Disposition::Supported(set.candidates()[0].clone()), None)
            }
            _ => (
                Disposition::Unsupported,
                Some(Diagnostic {
                    code: fsl_route::NegotiationOutcome::UnsupportedRequestedCapability.code(),
                    payload: Payload::Kind(item.kind),
                }),
            ),
        }
    }
}

const LIFECYCLE_UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"filament.value.complete/v1\";\n\
    profile e = \"filament.temporal.event-position.false-extension/v1\";\n\
    profile f = \"filament.temporal.fixed-sample.false-extension/v1\";\n\
    profile w = \"filament.temporal.timestamped-event.finite-window/v1\";\n\
    profile i = \"filament.temporal.infinite-trace/v1\";\n\
    dimension Time;\n\
    unit s : Time = rational(1, 1);\n\
    unit ms : Time = rational(1, 1000) * s;\n\
    record Reading { level: Int[0, 9]; }\n\
    temporal E using e over (r: Reading) clock \"events\" on origin { eventually[0,2] holds(r.level = 2) }\n\
    temporal S using f over (r: Reading) clock \"sample\" on origin { eventually[0,3] holds(r.level = 2) }\n\
    temporal T using w over (r: Reading) clock \"plant-ms\" on origin { eventually[0,5] holds(r.level > 0) }\n\
    temporal Reaches using i over (r: Reading) clock \"steps\" on origin { eventually holds(r.level = 2) }\n\
    temporal Never using i over (r: Reading) clock \"steps\" on origin { always holds(r.level < 2) }\n";

fn lifecycle_program(stem: &str) -> Value {
    json!({"source": {"file": format!("{stem}.native"), "authority": "agent-ix",
        "identity": format!("test:corpus:{stem}"), "document": "Lifecycle", "formal_revision": 1}})
}

fn level(level: i64) -> Value {
    json!({"sample": {"level": {"integer": level.to_string()}}})
}

fn lifecycle_clocks() -> Value {
    json!({
        "events": {"kind": "event_position", "sequence_authority": "events"},
        "sample": {"kind": "fixed_sample",
            "epoch": {"kind": "integer", "decimal": "0"},
            "period": {"kind": "integer", "decimal": "1"}, "unit": "s"},
        "plant-ms": {"kind": "timestamped_event", "timestamp_unit": "ms"},
    })
}

/// FR-286-AC-1, FR-286-AC-5, TC-770: a clause run, a monitor run and an
/// analyze run each carry `category` and `last_stage`, and the runner
/// compares both for each.
#[test]
#[trace("TC-770", "FR-286-AC-1", "FR-286-AC-5", "FR-286-AC-6")]
fn corpus_runner_compares_lifecycle_members_of_every_command_kind() {
    // A clause run over a temporal clause still pending at the end of the
    // trace: pending at S6a.
    let directory = tempfile::tempdir().unwrap();
    write_case(
        directory.path(),
        "clause",
        LIFECYCLE_UNIT,
        &json!({"verb":"run","exit_code":0,"category":"pending","last_stage":"S6a"}).to_string(),
    );
    let request = json!({"format":"native-run/1","request":{
        "models":[], "program": lifecycle_program("clause"), "clocks": lifecycle_clocks(),
        "clause":{"selection":{"temporal":{"name":"Reaches",
            "trace":{"finite":[level(0), level(2)]}}},"observations":{}}}});
    std::fs::write(
        directory.path().join("clause.request.json"),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();
    assert_eq!(run_directory(directory.path()).unwrap(), "flips_with: none");

    // A monitor run whose `Never` clause is violated: violation at S6a.
    let directory = tempfile::tempdir().unwrap();
    write_case(
        directory.path(),
        "monitor",
        LIFECYCLE_UNIT,
        &json!({"verb":"monitor","exit_code":10,"category":"violation","last_stage":"S6a"})
            .to_string(),
    );
    let mut records: Vec<String> = [0, 1, 2, 1]
        .iter()
        .map(|value| json!({"observation": level(*value)}).to_string())
        .collect();
    records.push(json!({"end": true}).to_string());
    std::fs::write(directory.path().join("records.jsonl"), records.join("\n")).unwrap();
    let request = json!({"format":"native-monitor/1","request":{
        "models":[], "program": lifecycle_program("monitor"), "clocks": lifecycle_clocks(),
        "clauses":[{"name":"Reaches"},{"name":"Never"}],
        "trace":{"kind":"finite"}, "records":"records.jsonl"}});
    std::fs::write(
        directory.path().join("monitor.request.json"),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();
    assert_eq!(run_directory(directory.path()).unwrap(), "flips_with: none");

    // An analyze run whose claim holds. FR-621 writes `last_stage` null.
    let directory = tempfile::tempdir().unwrap();
    let unit = format!(
        "{}temporal Q using t0 over (c: M::Counter) clock \"model-steps\" on origin {{ always holds(c.value <= 3) }}\n",
        std::str::from_utf8(include_bytes!("../../fsl-conformance/cases/counter-step.native"))
            .unwrap()
    );
    write_case(
        directory.path(),
        "analyze",
        &unit,
        &json!({"verb":"analyze","exit_code":0,"category":"success","last_stage":null}).to_string(),
    );
    std::fs::write(
        directory.path().join("package.json"),
        include_bytes!("../../fsl-conformance/cases/counter-step-package.json"),
    )
    .unwrap();
    std::fs::write(
        directory.path().join("v0.json"),
        include_bytes!("../../fsl-conformance/cases/counter-step-v0.json"),
    )
    .unwrap();
    let request = json!({"format":"native-analyze/1","request":{
        "program": {"source": {"file": "analyze.native", "authority": "test",
            "identity": "counter-step", "document": "CounterStep", "formal_revision": 1}},
        "models": [{"format": "semantic-ir/2.0.0", "source": {"file": "package.json",
            "authority": "test", "identity": "counter-step-package", "document": "Package",
            "formal_revision": 1}}],
        "documents": [{"file": "v0.json"}],
        "items": [{"claim": "Q"}]}});
    std::fs::write(
        directory.path().join("analyze.request.json"),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();
    assert_eq!(run_directory(directory.path()).unwrap(), "flips_with: none");
}
