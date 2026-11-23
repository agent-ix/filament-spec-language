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
            &command::CommandLimits::monitor(command::MonitorPreparationLimits {
                work: 16_777_216,
                storage: 268_435_456,
            }),
            &cancel,
        )
        .err(),
        Entry::Compile => {
            command::compile_outcome_with_overrides(path, &RequestOverrides::default()).err()
        }
        Entry::Analyze => command::analyze_with_providers_and_limits(
            path,
            RunLimits::default(),
            &command::CommandLimits::analyze(),
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

/// FR-1047-AC-2: an argument document is not a request envelope.
#[test]
#[trace("TC-3170", "FR-1047-AC-2")]
fn an_argv_document_is_not_refused_as_a_missing_envelope() {
    let (value, _) = refusal(Entry::Run, &json!({"argv": []}));
    assert!(value["details"].get("missing").is_none(), "{value}");
    assert!(
        !value["message"]
            .as_str()
            .unwrap()
            .contains("missing required"),
        "{value}"
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
    let mut example: Value =
        serde_json::from_str(&command::request_example("native-monitor/1", None).unwrap()).unwrap();
    assert!(example["request"]
        .as_object_mut()
        .unwrap()
        .remove("records")
        .is_none());
    assert_refusal(
        Entry::Run,
        &example,
        "invalid request: missing required member /request/records",
        &json!({"missing": ["/request/records"]}),
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

/// FR-1047-AC-6: an export lacking its profile keeps FR-1015's refusal.
#[test]
#[trace("TC-3170", "FR-1047-AC-6")]
fn an_export_lacking_its_profile_keeps_the_export_refusal() {
    let mut document = counter_export(Path::new("absent.native"));
    document["request"]["export"] = json!({"obligations": ["Capacity"]});
    let (value, exit) = refusal(Entry::Export(ExportSelection::Ordinary), &document);
    assert_eq!(exit, 20, "{value}");
    assert_eq!(value["code"], "invalid_runtime_input", "{value}");
    assert_eq!(
        value["details"],
        json!({"path": "/request/export/profile"}),
        "{value}"
    );
    assert!(value["details"].get("missing").is_none(), "{value}");
}

/// FR-1047: the format rule precedes the request, even for a present
/// `export` member.
#[test]
#[trace("TC-3170", "FR-1047-AC-5")]
fn the_format_is_refused_before_a_present_export_member() {
    assert_refusal(
        Entry::Export(ExportSelection::Ordinary),
        &json!({"format": "native-compile/1", "request": {"export": {}}}),
        "invalid request: format native-compile/1 is not read by export; export reads native-run/1",
        &json!({"path": "/format", "accepted": ["native-run/1"]}),
    );
}

fn entry_name(entry: Entry) -> &'static str {
    match entry {
        Entry::Run => "run",
        Entry::RunReader => "run-reader",
        Entry::Monitor => "monitor",
        Entry::Compile => "compile",
        Entry::Analyze => "analyze",
        Entry::Export(ExportSelection::Ordinary) => "export",
        Entry::Export(ExportSelection::Evidence) => "evidence",
    }
}

/// FR-1047 Behavior: an unknown or repeated member is refused for that
/// defect, ahead of any missing member.
#[test]
#[trace("TC-3170")]
fn an_unknown_or_repeated_member_wins_over_missing_members() {
    let directory = tempfile::tempdir().unwrap();
    let raw = [
        r#"{"format":"native-run/1","request":{},"x":1}"#,
        r#"{"format":"native-run/1","format":"native-run/1","request":{}}"#,
        r#"{"format":"native-run/1","request":{"models":[],"models":[]}}"#,
        r#"{"format":"native-run/1","request":{"z":1}}"#,
    ];
    for entry in [
        Entry::Run,
        Entry::RunReader,
        Entry::Compile,
        Entry::Analyze,
        Entry::Monitor,
        Entry::Export(ExportSelection::Ordinary),
        Entry::Export(ExportSelection::Evidence),
    ] {
        for (index, text) in raw.iter().enumerate() {
            let format = match entry {
                Entry::Compile => "native-compile/1",
                Entry::Analyze => "native-analyze/1",
                Entry::Monitor => "native-monitor/1",
                _ => "native-run/1",
            };
            let text = text.replace("native-run/1", format);
            let path = directory.path().join(format!("request-{index}.json"));
            std::fs::write(&path, &text).unwrap();
            let (value, _) = call(entry, &path).expect("refused");
            let length = format.len();
            let prefix = match entry {
                Entry::Analyze => "invalid analyze request member request: ",
                _ => "invalid request: ",
            };
            let message = value["message"].as_str().unwrap();
            let expected = match (index, entry) {
                (0, _) => Some(format!(
                    "{prefix}unknown field `x`, expected `format` or `request` at line 1 column {}",
                    29 + length
                )),
                (1 | 2, Entry::Run | Entry::Export(_)) => Some(format!(
                    "invalid clause request member request: duplicate object key {} at byte {}",
                    if index == 1 { "format" } else { "models" },
                    if index == 1 { 25 } else { 48 }
                )),
                (1, _) => Some(format!(
                    "{prefix}duplicate field `format` at line 1 column {}",
                    21 + length
                )),
                (2, _) => Some(format!(
                    "{prefix}duplicate field `models` at line 1 column 21"
                )),
                _ => None,
            };
            match expected {
                Some(expected) => assert_eq!(message, expected, "{text}"),
                None => assert!(
                    message.starts_with(&format!("{prefix}unknown field `z`, expected one of `")),
                    "{text}: {message}"
                ),
            }
            assert_eq!(value["code"], "invalid-request", "{text}: {value}");
            assert!(value["details"].get("missing").is_none(), "{text}: {value}");
        }
    }
}

/// A repeated name inside a subtree this requirement does not read leaves
/// the missing-members list intact.
#[test]
#[trace("TC-3170", "FR-1047-AC-1")]
fn a_repeat_inside_a_member_subtree_keeps_the_missing_list() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("request.json");
    std::fs::write(
        &path,
        r#"{"format":"native-compile/1","request":{"refinement_internals":{"a":1,"a":2}}}"#,
    )
    .unwrap();
    let (value, exit) = call(Entry::Compile, &path).expect("refused");
    assert_eq!(exit, 20, "{value}");
    assert_eq!(
        value["details"],
        json!({"missing": ["/request/program"]}),
        "{value}"
    );
}

/// A run reader keeps `items` as plain JSON: its repeated key is still the
/// repeated-member refusal, not the missing-members list.
#[test]
#[trace("TC-3170")]
fn a_repeat_in_a_plain_json_run_member_is_refused_as_a_repeat() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("request.json");
    std::fs::write(
        &path,
        r#"{"format":"native-run/1","request":{"items":{"a":1,"a":2}}}"#,
    )
    .unwrap();
    for entry in [Entry::Run, Entry::Export(ExportSelection::Ordinary)] {
        let (value, exit) = call(entry, &path).expect("refused");
        assert_eq!(exit, 20, "{value}");
        assert_eq!(value["code"], "invalid-request", "{value}");
        assert_eq!(
            value["message"],
            "invalid clause request member request: duplicate object key a at byte 51",
            "{value}"
        );
    }
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
        match (format, selection) {
            ("native-replay/1", _) => {
                // Over an empty directory the request's own byte provision
                // is read while decoding: the missing file is refused.
                let (value, exit) = call(entry, &path).expect("entry.fsl is missing");
                assert_eq!(exit, 20, "{value}");
                assert_eq!(value["stage"], "request", "{value}");
                assert!(
                    value["message"].as_str().unwrap().contains("entry.fsl"),
                    "{value}"
                );
                assert_ne!(value["code"], "invalid-request", "{value}");
                // With an empty entry.fsl the request decodes in full and
                // FR-098's recompile or package admission refuses it.
                std::fs::write(directory.path().join("entry.fsl"), "").unwrap();
                let (value, _) = call(entry, &path).expect("the example does not replay");
                // The recompile of the empty source refuses at stage `source`
                // (the spec does not pin the stage).
                assert_eq!(value["code"], "invalid_syntax", "{value}");
                assert_eq!(value["cause"], "unexpected-end", "{value}");
                assert_eq!(value["stage"], "source", "{value}");
                assert_eq!(
                    value["message"], "the recompile refused at stage source: expected `language`",
                    "{value}"
                );
            }
            (_, Some(ExportSelection::Evidence)) => {
                let result = command::export_with_limits_and_overrides(
                    &path,
                    ExportSelection::Evidence,
                    RunLimits::default(),
                    &RequestOverrides::default(),
                )
                .expect("the evidence example is accepted");
                let document = serde_json::to_value(result.outcome.unwrap()).unwrap();
                assert_eq!(
                    document["diagnostics"][0]["cause"], "unimplemented-meaning",
                    "{document}"
                );
            }
            _ => {
                let (value, _) = call(entry, &path)
                    .unwrap_or_else(|| panic!("{format} {selection:?} names a missing program"));
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
            let pointer = format!("/request/{name}");
            let expected = match one_of_group(format, selection, name) {
                Some(group) => json!(group
                    .iter()
                    .map(|member| format!("/request/{member}"))
                    .collect::<Vec<_>>()),
                None => json!(pointer),
            };
            assert_eq!(
                value["details"],
                json!({"missing": [expected]}),
                "{format} {selection:?} without {name}: {value}"
            );
        }
    }
}

/// The one-of group of the run, export and analyze readings that holds `name`.
fn one_of_group(
    format: &str,
    selection: Option<ExportSelection>,
    name: &str,
) -> Option<&'static [&'static str]> {
    let groups: &'static [&'static [&'static str]] = match (format, selection) {
        ("native-run/1", None) => &[&["program", "package"], &["call", "clause", "items"]],
        ("native-run/1", Some(ExportSelection::Ordinary)) | ("native-analyze/1", _) => {
            &[&["program", "package"]]
        }
        _ => &[],
    };
    groups.iter().copied().find(|group| group.contains(&name))
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
