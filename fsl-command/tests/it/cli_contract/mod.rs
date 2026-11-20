// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSL L3 CLI contract tests (QSL-2470 to QSL-2474, QSL-2477, QSL-2478).
//!
//! The `fsl` binary is built in filament-driver, which owns argv, `--help`,
//! `--format` and the process streams. This repository owns the typed
//! command surface the driver dispatches each verb to: `fsl_command::command`
//! (`compile_outcome_with_overrides`, `run_with_limits_and_overrides`,
//! `analyze_with_providers`, `monitor` and `run_with_overrides` for export),
//! the FR-255 settings operation behind `--limit`, and the FR-286/FR-644
//! machine documents. These tests call those entries exactly as the driver
//! does and assert the outcome the driver writes: the exit code of the
//! outcome's category and one machine document, validated against its
//! published schema.
//!
//! The one harness is here. Every verb's module builds its inputs in a
//! [`Case`], calls the verb through one of the functions below and asserts
//! the [`Outcome`] (or the monitor [`Stream`]).

use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use filament_exact::Cancel;
use filament_semantic_value::catalog::Category;
use fsl_analyze::provider::ProviderOptions;
use fsl_command::command::{self, MonitorProviders, RequestOverrides, RunLimits};
use fsl_replay::Operation;
use serde_json::Value;

mod analyze;
mod compile;
mod export;
mod machine_output;
mod monitor;
mod run;
mod surface;

/// One invocation's private working directory.
pub(crate) struct Case {
    directory: tempfile::TempDir,
}

impl Case {
    /// A fresh, empty working directory.
    pub(crate) fn new() -> Self {
        Self {
            directory: tempfile::tempdir().expect("a private working directory"),
        }
    }

    /// The path of `name` in the working directory.
    pub(crate) fn path(&self, name: &str) -> PathBuf {
        self.directory.path().join(name)
    }

    /// Write `bytes` to `name`, creating its parent directories.
    pub(crate) fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.path(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("the file's directory");
        }
        std::fs::write(&path, bytes).expect("the case file is written");
        path
    }

    /// Write `value`'s JSON text to `name`.
    pub(crate) fn json(&self, name: &str, value: &Value) -> PathBuf {
        self.write(name, value.to_string().as_bytes())
    }
}

/// What the driver writes for one document-producing verb: the exit code and
/// the one machine document, with its exact bytes.
#[derive(Debug)]
pub(crate) struct Outcome {
    pub(crate) exit: u8,
    pub(crate) bytes: Vec<u8>,
    pub(crate) document: Value,
    /// The checked package a successful compile returned.
    pub(crate) package: Option<Vec<u8>>,
}

impl Outcome {
    fn from_document(document: &fsl_replay::OutcomeDocument, exit: u8) -> Self {
        let bytes = document
            .to_bytes()
            .expect("the outcome document serializes");
        let value = serde_json::from_slice(&bytes).expect("the outcome is one JSON document");
        Self {
            exit,
            bytes,
            document: value,
            package: None,
        }
    }

    fn from_value(value: &Value, exit: u8) -> Self {
        let bytes = fsl_foundation::canonical_json::encode(value)
            .unwrap_or_else(|_| serde_json::to_vec(value).expect("a JSON value"));
        Self {
            exit,
            bytes,
            document: value.clone(),
            package: None,
        }
    }

    fn refusal(error: &command::RunError, operation: Operation) -> Self {
        let document = error.outcome_document(operation);
        Self::from_document(&document, error.category().exit_code())
    }

    /// Assert a `filament-outcome/1` document of `operation` in `category`
    /// with `exit`, valid against FR-286's published schema; returns it.
    pub(crate) fn expect(&self, operation: &str, category: &str, exit: u8) -> &Value {
        let document = &self.document;
        assert_eq!(self.exit, exit, "exit: {document}");
        assert_outcome_schema(document);
        assert_eq!(document["format"], "filament-outcome/1", "{document}");
        assert_eq!(document["operation"], operation, "{document}");
        assert_eq!(document["category"], category, "{document}");
        document
    }

    /// The first diagnostic with catalog `code`.
    pub(crate) fn diagnostic(&self, code: &str) -> &Value {
        self.document["diagnostics"]
            .as_array()
            .and_then(|all| all.iter().find(|diagnostic| diagnostic["code"] == code))
            .unwrap_or_else(|| panic!("no {code} diagnostic: {}", self.document))
    }

    /// Assert a refusal of `operation` with `exit` carrying diagnostic `code`.
    pub(crate) fn refused(&self, operation: &str, exit: u8, code: &str) -> &Value {
        let category = category_of(exit);
        self.expect(operation, category, exit);
        self.diagnostic(code)
    }
}

/// The FR-285 category spelling of `exit`.
pub(crate) fn category_of(exit: u8) -> &'static str {
    Category::ALL
        .into_iter()
        .find(|category| category.exit_code() == exit)
        .map(Category::as_str)
        .unwrap_or_else(|| panic!("exit {exit} names no category"))
}

fn compiled(text: &str) -> jsonschema::JSONSchema {
    let value: Value = serde_json::from_str(text).expect("schema JSON is valid");
    jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .compile(&value)
        .expect("schema compiles")
}

fn check(schema: &jsonschema::JSONSchema, name: &str, value: &Value) {
    if let Err(errors) = schema.validate(value) {
        let errors: Vec<String> = errors
            .map(|error| format!("{} at {}", error, error.instance_path))
            .collect();
        panic!("{name} schema rejects the document: {errors:?}\n{value}");
    }
}

/// Assert `value` validates against `urn:filament:outcome:1` (FR-286).
pub(crate) fn assert_outcome_schema(value: &Value) {
    static SCHEMA: OnceLock<jsonschema::JSONSchema> = OnceLock::new();
    let schema = SCHEMA.get_or_init(|| compiled(fsl_replay::outcome::OUTCOME_SCHEMA_1));
    check(schema, "filament-outcome/1", value);
}

/// Assert `value` validates against `urn:filament:monitor-event:1` (FR-644).
pub(crate) fn assert_event_schema(value: &Value) {
    static SCHEMA: OnceLock<jsonschema::JSONSchema> = OnceLock::new();
    let schema = SCHEMA.get_or_init(|| compiled(command::monitor::MONITOR_EVENT_SCHEMA_1));
    check(schema, "filament.monitor-event/1", value);
}

/// Assert `value` validates against `urn:filament:monitor-summary:1` (FR-644).
pub(crate) fn assert_summary_schema(value: &Value) {
    static SCHEMA: OnceLock<jsonschema::JSONSchema> = OnceLock::new();
    let schema = SCHEMA.get_or_init(|| compiled(command::monitor::MONITOR_SUMMARY_SCHEMA_1));
    check(schema, "filament.monitor-summary/1", value);
}

/// `fsl compile <request>`: the driver's entry
/// `compile_outcome_with_overrides`, or with `limits` (the `--limit`
/// request and dependent byte ceilings) `compile_outcome_with_limits_and_overrides`.
pub(crate) fn compile(
    path: &Path,
    limits: Option<RunLimits>,
    overrides: &RequestOverrides<'_>,
) -> Outcome {
    let result = match limits {
        None => command::compile_outcome_with_overrides(path, overrides),
        Some(limits) => command::compile_outcome_with_limits_and_overrides(path, limits, overrides),
    };
    match result {
        Ok(outcome) => {
            let mut written = Outcome::from_document(outcome.document(), 0);
            written.package = Some(outcome.package_bytes().to_vec());
            written
        }
        Err(error) => Outcome::refusal(&error, Operation::Package),
    }
}

/// `fsl run <request>` through `run_with_limits_and_overrides`: the run's
/// outcome document when it has one, else its native result document.
pub(crate) fn run(path: &Path, limits: RunLimits, overrides: &RequestOverrides<'_>) -> Outcome {
    match command::run_with_limits_and_overrides(
        path,
        limits,
        overrides,
        &crate::negotiation::Fr290,
    ) {
        Ok(result) => match &result.outcome {
            Some(document) => Outcome::from_document(document, result.exit_code),
            None => Outcome::from_value(result.value.as_value(), result.exit_code),
        },
        Err(error) => Outcome::refusal(&error, Operation::Execute),
    }
}

/// `fsl analyze <request>` through `analyze_with_providers_and_limits` with
/// no configured provider, under `options` and `cancel`.
pub(crate) fn analyze_under(
    path: &Path,
    limits: RunLimits,
    options: &ProviderOptions,
    cancel: &Cancel,
) -> Outcome {
    match command::analyze_with_providers_and_limits(
        path,
        limits,
        &crate::negotiation::Fr290,
        Vec::new(),
        options,
        cancel,
    ) {
        Ok(result) => Outcome::from_value(result.value.as_value(), result.exit_code),
        Err(error) => Outcome::refusal(&error, Operation::Analyze),
    }
}

/// [`analyze_under`] with the defaults the driver uses.
pub(crate) fn analyze(path: &Path) -> Outcome {
    analyze_under(
        path,
        RunLimits::default(),
        &ProviderOptions::default(),
        &Cancel::new(),
    )
}

/// `fsl export <request>` through `run_with_overrides` with `--output` and
/// `--limit` as request overrides: the export's outcome document.
pub(crate) fn export(path: &Path, overrides: &RequestOverrides<'_>) -> Outcome {
    match command::run_with_overrides(path, overrides, &crate::negotiation::Fr290) {
        Ok(result) => {
            let document = result
                .outcome
                .as_ref()
                .expect("an export result carries its outcome document");
            Outcome::from_document(document, result.exit_code)
        }
        Err(error) => Outcome::refusal(&error, Operation::Export),
    }
}

/// What `fsl monitor` writes: one line per event, then the summary, with
/// the summary's exit code; or, for a refusal before any record is read,
/// the one `monitor` outcome document.
#[derive(Debug)]
pub(crate) enum Stream {
    Lines {
        exit: u8,
        events: Vec<Value>,
        summary: Value,
        bytes: Vec<u8>,
    },
    Refused(Outcome),
}

impl Stream {
    /// Assert a completed stream with `exit`; every line validates against
    /// its FR-644 schema and is its own canonical encoding.
    pub(crate) fn completed(&self, exit: u8) -> (&[Value], &Value) {
        match self {
            Self::Lines {
                exit: actual,
                events,
                summary,
                ..
            } => {
                assert_eq!(*actual, exit, "{summary}");
                for event in events {
                    assert_event_schema(event);
                }
                assert_summary_schema(summary);
                (events, summary)
            }
            Self::Refused(outcome) => panic!("refused: {}", outcome.document),
        }
    }

    /// Assert a refusal before any record was read.
    pub(crate) fn refused(&self) -> &Outcome {
        match self {
            Self::Refused(outcome) => outcome,
            Self::Lines { summary, .. } => panic!("not refused: {summary}"),
        }
    }

    /// The bytes the driver writes to stdout for a completed stream.
    pub(crate) fn bytes(&self) -> &[u8] {
        match self {
            Self::Lines { bytes, .. } => bytes,
            Self::Refused(outcome) => &outcome.bytes,
        }
    }
}

/// `fsl monitor <request>` over the record stream `records` (the bytes of
/// `--input` or stdin), split into lines as the driver splits them.
pub(crate) fn monitor_under(path: &Path, records: &[u8], cancel: &Cancel) -> Stream {
    let lines = std::io::Cursor::new(records.to_vec()).split(b'\n');
    let mut events = Vec::new();
    let mut bytes = Vec::new();
    let summary = command::monitor(
        path,
        lines,
        &mut |event| {
            let line =
                fsl_foundation::canonical_json::encode(event.as_value()).expect("an event encodes");
            assert_eq!(line, event.to_string().into_bytes(), "canonical event line");
            bytes.extend_from_slice(&line);
            bytes.push(b'\n');
            events.push(event.as_value().clone());
        },
        MonitorProviders {
            negotiate: &crate::negotiation::Fr290,
            providers: Vec::new(),
            options: &ProviderOptions::default(),
        },
        RunLimits::default(),
        cancel,
    );
    match summary {
        Ok(summary) => {
            let line = fsl_foundation::canonical_json::encode(summary.as_value())
                .expect("the summary encodes");
            bytes.extend_from_slice(&line);
            bytes.push(b'\n');
            Stream::Lines {
                exit: summary.exit_code,
                events,
                summary: summary.as_value().clone(),
                bytes,
            }
        }
        Err(error) => {
            assert!(events.is_empty(), "a refusal writes no event: {events:?}");
            Stream::Refused(Outcome::refusal(&error, Operation::Monitor))
        }
    }
}

/// [`monitor_under`] with a handle that is never cancelled.
pub(crate) fn monitor(path: &Path, records: &[u8]) -> Stream {
    monitor_under(path, records, &Cancel::new())
}

/// The bytes of a NUL-free, invalid UTF-8 source for FR-001-AC-3.
pub(crate) const INVALID_UTF8: &[u8] = b"language \"ix:native\" edition \"1-draft\";\n\xff\xfe\n";
