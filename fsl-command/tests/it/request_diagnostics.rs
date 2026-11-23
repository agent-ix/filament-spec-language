// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1047 (TC-3170): request refusals name every missing member and every
//! accepted format, and every request example is read back by its entry.

use std::path::Path;

use fsl_command::command::{
    self, ExportSelection, MonitorProviders, RequestExampleError, RequestOverrides, RunLimits,
};
use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::negotiation::Fr290;

#[derive(Clone, Copy)]
enum Entry {
    Run,
    RunReader,
    Monitor,
    Compile,
    Analyze,
    Export(ExportSelection),
}

/// The refusal `entry` gives the request file at `path`, as its envelope and
/// exit code; `None` when the entry accepts the request.
fn call(entry: Entry, path: &Path) -> Option<(Value, u8)> {
    let options = fsl_analyze::provider::ProviderOptions::default();
    let cancel = filament_exact::Cancel::new();
    let error = match entry {
        Entry::RunReader => command::prepare_run(path, RunLimits::default()).err(),
        Entry::Run => {
            command::run_with_providers(path, &Fr290, Vec::new(), &options, &cancel).err()
        }
        Entry::Monitor => command::monitor(
            path,
            std::iter::empty(),
            &mut |_| {},
            MonitorProviders {
                negotiate: &Fr290,
                providers: Vec::new(),
                options: &options,
            },
            RunLimits::default(),
            &cancel,
        )
        .err(),
        Entry::Compile => {
            command::compile_outcome_with_overrides(path, &RequestOverrides::default()).err()
        }
        Entry::Analyze => command::analyze_with_providers_and_limits(
            path,
            RunLimits::default(),
            &Fr290,
            Vec::new(),
            &options,
            &cancel,
        )
        .err(),
        Entry::Export(selection) => command::export_with_limits_and_overrides(
            path,
            selection,
            RunLimits::default(),
            &RequestOverrides::default(),
        )
        .err(),
    }?;
    Some((error.value().unwrap(), error.category().exit_code()))
}

fn refusal(entry: Entry, document: &Value) -> (Value, u8) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("request.json");
    std::fs::write(&path, document.to_string()).unwrap();
    call(entry, &path).expect("the request is refused")
}

/// A request-stage `invalid-request` refusal with exactly `message` and
/// `details`, carrying no line or column.
fn assert_refusal(entry: Entry, document: &Value, message: &str, details: &Value) {
    let (value, exit) = refusal(entry, document);
    assert_eq!(exit, 20, "{value}");
    assert_eq!(value["code"], "invalid-request", "{value}");
    assert_eq!(value["stage"], "request", "{value}");
    assert_eq!(value["message"], message, "{value}");
    assert_eq!(&value["details"], details, "{value}");
    let text = value.to_string();
    assert!(
        !text.contains("\"line\"") && !text.contains("\"column\""),
        "{value}"
    );
    assert!(!text.contains("unknown_wire"), "{value}");
}

const RUN: &str = "native-run/1";

/// FR-1047-AC-1: every missing member and group at once, as pointers.
#[test]
#[trace("TC-3170", "FR-1047-AC-1")]
fn a_run_request_lists_every_missing_member() {
    assert_refusal(
        Entry::Run,
        &json!({"format": RUN, "request": {}}),
        "invalid request: missing required members /request/models, /request/program | /request/package, /request/call | /request/clause | /request/items",
        &json!({"missing": ["/request/models", ["/request/program", "/request/package"], ["/request/call", "/request/clause", "/request/items"]]}),
    );
}

/// FR-1047-AC-2: the envelope is read first.
#[test]
#[trace("TC-3170", "FR-1047-AC-2")]
fn a_request_without_its_envelope_members_lists_only_those() {
    assert_refusal(
        Entry::Run,
        &json!({}),
        "invalid request: missing required members /format, /request",
        &json!({"missing": ["/format", "/request"]}),
    );
    assert_refusal(
        Entry::Run,
        &json!({"format": RUN}),
        "invalid request: missing required member /request",
        &json!({"missing": ["/request"]}),
    );
    assert_refusal(
        Entry::Run,
        &json!({"format": "bogus/1"}),
        "invalid request: format bogus/1 is not read by run; run reads native-run/1, native-monitor/1, native-replay/1",
        &json!({"path": "/format", "accepted": ["native-run/1", "native-monitor/1", "native-replay/1"]}),
    );
}

/// FR-1047-AC-3: the run entry dispatches a monitor request to its reader.
#[test]
#[trace("TC-3170", "FR-1047-AC-3")]
fn a_monitor_request_lists_the_monitor_members() {
    let document = json!({"format": "native-monitor/1", "request": {}});
    assert_refusal(
        Entry::Run,
        &document,
        "invalid request: missing required members /request/models, /request/program, /request/clauses, /request/trace, /request/records",
        &json!({"missing": ["/request/models", "/request/program", "/request/clauses", "/request/trace", "/request/records"]}),
    );
    assert_refusal(
        Entry::Monitor,
        &document,
        "invalid request: missing required members /request/models, /request/program, /request/clauses, /request/trace",
        &json!({"missing": ["/request/models", "/request/program", "/request/clauses", "/request/trace"]}),
    );
    let replay = json!({"format": "native-replay/1", "request": {}});
    let fourteen: Vec<String> = fsl_command::replay_request::MEMBERS
        .iter()
        .map(|member| format!("/request/{member}"))
        .collect();
    let (value, exit) = refusal(Entry::Run, &replay);
    assert_eq!(exit, 20, "{value}");
    assert_eq!(value["stage"], "request", "{value}");
    assert_eq!(value["details"], json!({"missing": fourteen}), "{value}");
}

/// FR-1047-AC-4: missing members at depth, before any file is read.
#[test]
#[trace("TC-3170", "FR-1047-AC-4")]
fn missing_members_are_listed_at_every_depth() {
    assert_refusal(
        Entry::Run,
        &json!({"format": RUN, "request": {
            "models": [{"source": {"file": "m.json"}}],
            "call": {"function": "main", "arguments": []},
            "program": {"source": {"file": "p.fsl"}}}}),
        "invalid request: missing required members /request/models/0/format, /request/models/0/source/authority, /request/models/0/source/identity, /request/models/0/source/document, /request/models/0/source/formal_revision, /request/program/source/authority, /request/program/source/identity, /request/program/source/document, /request/program/source/formal_revision",
        &json!({"missing": [
            "/request/models/0/format", "/request/models/0/source/authority",
            "/request/models/0/source/identity", "/request/models/0/source/document",
            "/request/models/0/source/formal_revision",
            "/request/program/source/authority", "/request/program/source/identity",
            "/request/program/source/document", "/request/program/source/formal_revision"]}),
    );
}

/// FR-1047-AC-5: a format the entry does not read names the formats it does.
#[test]
#[trace("TC-3170", "FR-1047-AC-5")]
fn a_format_the_entry_does_not_read_names_the_accepted_formats() {
    let cases = [
        (
            Entry::Run,
            "native-analyze/1",
            "invalid request: format native-analyze/1 is not read by run; run reads native-run/1, native-monitor/1, native-replay/1",
            json!(["native-run/1", "native-monitor/1", "native-replay/1"]),
        ),
        (
            Entry::RunReader,
            "native-monitor/1",
            "invalid request: format native-monitor/1 is not read by run-reader; run-reader reads native-run/1",
            json!(["native-run/1"]),
        ),
        (
            Entry::Monitor,
            "native-run/1",
            "invalid request: format native-run/1 is not read by monitor; monitor reads native-monitor/1",
            json!(["native-monitor/1"]),
        ),
        (
            Entry::Analyze,
            "native-run/1",
            "invalid request: format native-run/1 is not read by analyze; analyze reads native-analyze/1",
            json!(["native-analyze/1"]),
        ),
        (
            Entry::Compile,
            "native-run/1",
            "invalid request: format native-run/1 is not read by compile; compile reads native-compile/1",
            json!(["native-compile/1"]),
        ),
        (
            Entry::Export(ExportSelection::Ordinary),
            "native-export/1",
            "invalid request: format native-export/1 is not read by export; export reads native-run/1",
            json!(["native-run/1"]),
        ),
        (
            Entry::Run,
            "bogus/1",
            "invalid request: format bogus/1 is not read by run; run reads native-run/1, native-monitor/1, native-replay/1",
            json!(["native-run/1", "native-monitor/1", "native-replay/1"]),
        ),
    ];
    for (entry, format, message, accepted) in cases {
        assert_refusal(
            entry,
            &json!({"format": format, "request": {}}),
            message,
            &json!({"path": "/format", "accepted": accepted}),
        );
    }
}

fn counter_export(source: &Path) -> Value {
    json!({"format": RUN, "request": {
        "models": [],
        "program": {"source": {"file": source, "authority": "export-contracts",
            "identity": "contracts:bin-capacity", "document": "BinCapacity", "formal_revision": 1}},
        "export": {"profile": "filament.output.tla2/v1", "obligations": ["Capacity"]}}})
}

/// FR-1047-AC-6: the export member's name and shape, not a circular message.
#[test]
#[trace("TC-3170", "FR-1047-AC-6")]
fn a_missing_export_member_names_its_shape() {
    let mut ordinary = counter_export(Path::new("absent.native"));
    ordinary["request"]
        .as_object_mut()
        .unwrap()
        .remove("export");
    assert_refusal(
        Entry::Export(ExportSelection::Ordinary),
        &ordinary,
        "invalid request: missing required member /request/export: export reads a native-run/1 request whose request holds an export object with at least profile and obligations",
        &json!({"missing": ["/request/export"]}),
    );
    let evidence = counter_export(Path::new("absent.native"));
    assert_refusal(
        Entry::Export(ExportSelection::Evidence),
        &evidence,
        "invalid request: missing required member /request/export_evidence: export evidence reads a native-run/1 request whose request holds an export_evidence object with package, obligation, tool and artifact",
        &json!({"missing": ["/request/export_evidence"]}),
    );
}

/// The seven inputs of FR-1047-AC-7 with the entry that reads each.
fn example_inputs() -> Vec<(&'static str, Option<ExportSelection>, Entry)> {
    vec![
        ("native-run/1", None, Entry::Run),
        ("native-compile/1", None, Entry::Compile),
        ("native-analyze/1", None, Entry::Analyze),
        ("native-monitor/1", None, Entry::Monitor),
        ("native-replay/1", None, Entry::Run),
        (
            "native-run/1",
            Some(ExportSelection::Ordinary),
            Entry::Export(ExportSelection::Ordinary),
        ),
        (
            "native-run/1",
            Some(ExportSelection::Evidence),
            Entry::Export(ExportSelection::Evidence),
        ),
    ]
}

/// FR-1047-AC-7: every example is read back by its entry, and each member of
/// it is required.
#[test]
#[trace("TC-3170", "FR-1047-AC-7")]
fn every_request_example_is_read_back_and_each_member_is_required() {
    for (format, selection, entry) in example_inputs() {
        let text = command::request_example(format, selection).unwrap();
        assert_eq!(
            text,
            command::request_example(format, selection).unwrap(),
            "{format}"
        );
        assert!(!text.ends_with('\n'), "{text}");
        let document: Value = serde_json::from_str(&text).unwrap();
        assert!(text.starts_with("{\n  \"format\": "), "{text}");
        assert!(
            text.lines().all(|line| {
                let indent = line.len() - line.trim_start().len();
                indent % 2 == 0
            }),
            "two-space indentation: {text}"
        );
        assert!(text.contains("\n  \"request\": {"), "{text}");
        let members: Vec<&str> = document
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert!(members == ["format", "request"] || members == ["request", "format"]);
        assert!(text.find("\"format\"") < text.find("\"request\""), "{text}");
        assert_eq!(document["format"], format);

        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("request.json");
        std::fs::write(&path, &text).unwrap();
        if let Some((value, _)) = call(entry, &path) {
            if format == "native-replay/1" {
                // A replay request is admitted against its package's
                // declarations; the example holds no real package, so that
                // admission, and no other, refuses it.
                assert_eq!(value["code"], "invalid_runtime_input", "{value}");
                assert_eq!(value["stage"], "request", "{value}");
            } else {
                assert_eq!(value["stage"], "file", "{format} {selection:?}: {value}");
                assert_eq!(value["code"], "io-error", "{value}");
                assert_eq!(
                    value["details"]["path"],
                    json!(directory.path().join("program.fsl")),
                    "{value}"
                );
            }
        }

        for (name, _) in document["request"].as_object().unwrap() {
            let mut reduced = document.clone();
            reduced["request"].as_object_mut().unwrap().remove(name);
            let (value, exit) = refusal(entry, &reduced);
            assert_eq!(exit, 20, "{format} without {name}: {value}");
            assert_eq!(value["code"], "invalid-request", "{value}");
            let missing = value["details"]["missing"]
                .as_array()
                .unwrap_or_else(|| panic!("{format} without {name}: no details.missing: {value}"));
            let pointer = json!(format!("/request/{name}"));
            let group = |entry: &Value| {
                entry
                    .as_array()
                    .is_some_and(|group| group.contains(&pointer))
            };
            assert!(
                missing.contains(&pointer) || missing.iter().any(group),
                "{format} {selection:?} without {name}: {value}"
            );
        }
    }
}

/// FR-1047-AC-8: an unknown format and a selection for another format.
#[test]
#[trace("TC-3170", "FR-1047-AC-8")]
fn the_example_entry_refuses_an_unknown_format_and_a_misplaced_selection() {
    let unknown = command::request_example("native-export/1", None).unwrap_err();
    assert_eq!(
        unknown,
        RequestExampleError::UnknownFormat {
            format: "native-export/1".to_owned(),
            known: vec![
                "native-run/1",
                "native-compile/1",
                "native-analyze/1",
                "native-monitor/1",
                "native-replay/1"
            ],
        }
    );
    assert_eq!(
        unknown.to_string(),
        "unknown request format native-export/1; request formats are native-run/1, native-compile/1, native-analyze/1, native-monitor/1, native-replay/1"
    );
    let misplaced =
        command::request_example("native-compile/1", Some(ExportSelection::Ordinary)).unwrap_err();
    assert_eq!(
        misplaced,
        RequestExampleError::SelectionNeedsRunFormat {
            format: "native-compile/1".to_owned()
        }
    );
    assert_eq!(
        misplaced.to_string(),
        "an export selection reads native-run/1, not native-compile/1"
    );
    for error in [&unknown, &misplaced] {
        assert_eq!(error.code().as_str(), "invalid-request");
    }
}
