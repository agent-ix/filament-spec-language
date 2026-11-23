// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2470: `fsl compile <request>` at the command surface the driver calls
//! (`compile_outcome_with_overrides`, and `..._with_limits_and_overrides`
//! for `--limit`): success, every refusal class, the `--output`, `--clocks`
//! and `--limit` overrides, and odd request and source input.

use fsl_command::command::{RequestOverrides, RunLimits};
use fsl_replay::spine::SpineLimits;
use ix_trace_rs::trace;
use serde_json::value::RawValue;
use serde_json::{json, Value};

use super::{compile, Case, Outcome, INVALID_UTF8};

const SOURCE: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"filament.value.complete/v1\";\n\
    record Parcel { pieces: Int[0, 31]; }\n\
    function Total using v(pieces: Integer): Integer pure { pieces + 2 }\n";

const CLOCKED: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"filament.value.complete/v1\";\n\
    profile e = \"filament.temporal.event-position.false-extension/v1\";\n\
    record Reading { level: Int[0, 9]; }\n\
    temporal E using e over (r: Reading) clock \"events\" on origin { eventually[0,2] holds(r.level = 2) }\n";

fn request(file: &str) -> Value {
    json!({"format": "native-compile/1", "request": {"models": [], "program": {"source": {
        "file": file, "authority": "compile-contracts", "identity": "parcel",
        "document": "ParcelCompile", "formal_revision": 1}}}})
}

/// A case holding `source` as `parcel.fsl` and its compile request.
fn parcel(source: &[u8]) -> (Case, std::path::PathBuf) {
    let case = Case::new();
    case.write("parcel.fsl", source);
    let path = case.json("compile.json", &request("parcel.fsl"));
    (case, path)
}

fn plain(path: &std::path::Path) -> Outcome {
    compile(path, None, &RequestOverrides::default())
}

fn clocks() -> Value {
    json!({"events": {"kind": "event_position", "sequence_authority": "events"}})
}

/// FR-027, FR-286: a compile writes one `package` outcome at S4, success,
/// exit 0, with the checked package handed back beside it.
#[test]
#[trace("FR-027", "FR-286-AC-25", "FR-285-AC-1")]
fn compile_succeeds_with_one_package_outcome_and_a_checked_package() {
    let (_case, path) = parcel(SOURCE.as_bytes());
    let outcome = plain(&path);
    let document = outcome.expect("package", "success", 0);
    assert_eq!(document["last_stage"], "S4", "{document}");
    assert_eq!(document["diagnostics"], json!([]), "{document}");
    let package: Value =
        serde_json::from_slice(outcome.package.as_ref().expect("package bytes")).unwrap();
    assert_eq!(package["contract_version"], "filament.checked-package/v2");
}

/// FR-810-AC-3 (`--output`): the override writes the package byte-equal to
/// the returned package; FR-810-AC-2: a second run over the now-existing
/// path refuses `invalid-request`, exit 20, and leaves its bytes unchanged.
#[test]
#[trace("FR-810-AC-2", "FR-810-AC-3")]
fn output_override_writes_the_package_and_never_overwrites() {
    let (case, path) = parcel(SOURCE.as_bytes());
    let target = case.path("parcel.package.json");
    let overrides = RequestOverrides {
        output: Some(&target),
        ..RequestOverrides::default()
    };
    let written = compile(&path, None, &overrides);
    written.expect("package", "success", 0);
    let bytes = std::fs::read(&target).expect("the package file");
    assert_eq!(Some(&bytes), written.package.as_ref());

    let again = compile(&path, None, &overrides);
    let diagnostic = again.refused("package", 20, "invalid-request");
    assert!(
        diagnostic.to_string().contains("parcel.package.json"),
        "{diagnostic}"
    );
    assert_eq!(std::fs::read(&target).unwrap(), bytes);
}

/// FR-810: `--output` beside a request that carries `output` refuses
/// `invalid-request`, exit 20, and writes neither path.
#[test]
#[trace("FR-810")]
fn output_override_beside_an_output_member_refuses() {
    let case = Case::new();
    case.write("parcel.fsl", SOURCE.as_bytes());
    let mut both = request("parcel.fsl");
    both["request"]["output"] = json!({"package": "member.json"});
    let path = case.json("compile.json", &both);
    let target = case.path("flag.json");
    let overrides = RequestOverrides {
        output: Some(&target),
        ..RequestOverrides::default()
    };
    let outcome = compile(&path, None, &overrides);
    outcome.refused("package", 20, "invalid-request");
    assert!(!target.exists());
    assert!(!case.path("member.json").exists());
}

/// FR-640-AC-13 (`--clocks`): the override compiles the package the member
/// compiles; FR-640-AC-14: both refuse `invalid-request` at stage `request`.
#[test]
#[trace("FR-640-AC-13", "FR-640-AC-14")]
fn clocks_override_equals_the_member_and_both_refuse() {
    let case = Case::new();
    case.write("parcel.fsl", CLOCKED.as_bytes());
    let mut member = request("parcel.fsl");
    member["request"]["clocks"] = clocks();
    let member = case.json("member.json", &member);
    let bare = case.json("bare.json", &request("parcel.fsl"));
    let supplied = RawValue::from_string(clocks().to_string()).unwrap();
    let overrides = RequestOverrides {
        clocks: Some(&supplied),
        ..RequestOverrides::default()
    };

    let expected = plain(&member);
    expected.expect("package", "success", 0);
    let flagged = compile(&bare, None, &overrides);
    flagged.expect("package", "success", 0);
    assert_eq!(flagged.package, expected.package);
    assert_eq!(flagged.bytes, expected.bytes);

    let missing = plain(&bare);
    assert_eq!(missing.exit, 20, "{}", missing.document);
    missing.expect("package", "refusal", 20);

    let twice = compile(&member, None, &overrides);
    twice.refused("package", 20, "invalid-request");
    assert!(twice.document["last_stage"].is_null(), "{}", twice.document);
}

/// FR-869-AC-1 (`--limit` request bytes): one byte over the request ceiling
/// stops at intake with exit 22 naming `request_bytes`; the raised ceiling
/// compiles.
#[test]
#[trace("FR-869-AC-1")]
fn request_byte_ceiling_is_reached_and_raised() {
    let (_case, path) = parcel(SOURCE.as_bytes());
    let size = std::fs::metadata(&path).unwrap().len() as usize;
    let limits = |request_bytes| RunLimits {
        request_bytes,
        ..RunLimits::default()
    };
    let over = compile(&path, Some(limits(size - 1)), &RequestOverrides::default());
    over.expect("package", "incomplete", 22);
    assert!(
        over.document.to_string().contains("request_bytes"),
        "{}",
        over.document
    );
    let fits = compile(&path, Some(limits(size)), &RequestOverrides::default());
    fits.expect("package", "success", 0);
}

/// FR-001-AC-4 (`--limit s1.input_bytes=N`): a source over the S1 byte
/// ceiling refuses `stage_limit_exceeded` naming the setting, exit 22; the
/// ceiling at the source's size compiles.
#[test]
#[trace("FR-001-AC-4", "FR-869-AC-2")]
fn source_byte_stage_limit_is_reached_and_raised() {
    let (_case, path) = parcel(SOURCE.as_bytes());
    let mut limits = SpineLimits::default();
    limits.source.source_bytes = SOURCE.len() - 1;
    let overrides = RequestOverrides {
        limits: Some(&limits),
        ..RequestOverrides::default()
    };
    let over = compile(&path, Some(RunLimits::default()), &overrides);
    let diagnostic = over.refused("package", 22, "stage_limit_exceeded");
    assert!(
        diagnostic.to_string().contains("s1.input_bytes"),
        "{diagnostic}"
    );

    limits.source.source_bytes = SOURCE.len();
    let overrides = RequestOverrides {
        limits: Some(&limits),
        ..RequestOverrides::default()
    };
    compile(&path, Some(RunLimits::default()), &overrides).expect("package", "success", 0);
}

/// FR-001-AC-3, AC-8: invalid UTF-8 and a NUL byte in the source refuse with
/// a source diagnostic, exit 20, and no package.
#[test]
#[trace("FR-001-AC-3", "FR-001-AC-8")]
fn invalid_utf8_and_nul_sources_refuse() {
    let mut nul = SOURCE.as_bytes().to_vec();
    nul.extend_from_slice(b"\0\n");
    for source in [INVALID_UTF8.to_vec(), nul] {
        let (_case, path) = parcel(&source);
        let outcome = plain(&path);
        outcome.expect("package", "refusal", 20);
        assert!(outcome.package.is_none());
        let diagnostics = outcome.document["diagnostics"].as_array().unwrap();
        assert!(!diagnostics.is_empty(), "{}", outcome.document);
    }
}

/// FR-286-AC-19: an ill-typed unit refuses `ill_typed` at S3, exit 20.
#[test]
#[trace("FR-286-AC-19", "FR-285-AC-3")]
fn ill_typed_source_refuses_at_s3() {
    let source = "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"filament.value.complete/v1\";\n\
        function bad using v(): Int[0, 9] pure { true }\n";
    let (_case, path) = parcel(source.as_bytes());
    let outcome = plain(&path);
    outcome.refused("package", 20, "ill_typed");
    assert_eq!(outcome.document["last_stage"], "S3", "{}", outcome.document);
}

/// FR-001 file errors: a missing, a directory and an empty request operand,
/// and a missing source file, each refuse with exit 20 and one diagnostic.
#[test]
#[trace("FR-001", "FR-285-AC-1")]
fn missing_empty_and_directory_operands_refuse() {
    let case = Case::new();
    std::fs::create_dir_all(case.path("folder")).unwrap();
    let empty = case.write("empty.json", b"");
    let missing_source = case.json("compile.json", &request("absent.fsl"));
    for path in [
        case.path("absent.json"),
        case.path("folder"),
        empty,
        missing_source,
    ] {
        let outcome = plain(&path);
        outcome.expect("package", "refusal", 20);
        assert_eq!(
            outcome.document["diagnostics"].as_array().map(Vec::len),
            Some(1),
            "{}: {}",
            path.display(),
            outcome.document
        );
    }
}

/// FR-286: a malformed request, an unknown format and an unknown request
/// member each refuse `invalid-request`, exit 20, before any stage.
#[test]
#[trace("FR-286")]
fn malformed_requests_refuse_invalid_request() {
    let case = Case::new();
    case.write("parcel.fsl", SOURCE.as_bytes());
    let mut unknown_member = request("parcel.fsl");
    unknown_member["request"]["extra"] = json!(true);
    let mut wrong_format = request("parcel.fsl");
    wrong_format["format"] = json!("native-compile/9");
    let malformed = case.write("malformed.json", b"{\"format\": ");
    for (path, code) in [
        (malformed, "invalid-request"),
        (case.json("member.json", &unknown_member), "invalid-request"),
        // FR-1047: a format the entry does not read is `invalid-request`.
        (case.json("format.json", &wrong_format), "invalid-request"),
    ] {
        let outcome = plain(&path);
        outcome.refused("package", 20, code);
        assert!(
            outcome.document["last_stage"].is_null(),
            "{}",
            outcome.document
        );
    }
}

/// The text `fsl compile` writes for the compile of `path`: FR-865's renderer over the
/// document alone, with the sources the compile admitted.
fn text_of(path: &std::path::Path, overrides: &RequestOverrides<'_>) -> String {
    let outcome = fsl_command::command::compile_outcome_with_overrides(path, overrides)
        .expect("the compile succeeds");
    let sources = fsl_command::render_sources(outcome.sources());
    fsl_inspect::render_outcome(
        outcome.document(),
        fsl_inspect::RenderForm::Text,
        &sources,
        fsl_replay::RenderLimits::default(),
        &filament_exact::Cancel::new(),
    )
    .expect("the document renders")
    .value()
    .as_str()
    .to_owned()
}

/// FR-286-AC-39, FR-810-AC-6: the compile document names the package's length
/// and the path of each file written, joined to the request file's directory.
#[test]
#[trace("TC-3203", "FR-286-AC-39", "FR-810-AC-6")]
fn compile_document_names_the_package_and_where_it_was_written() {
    let case = Case::new();
    case.write("reqs/parcel.fsl", SOURCE.as_bytes());
    let mut member = request("parcel.fsl");
    member["request"]["output"] = json!({"package": "spine.package.json"});
    let with_output = case.json("reqs/with-output.json", &member);
    let written = plain(&with_output);
    let document = written.expect("package", "success", 0);
    let bytes = written.package.as_ref().expect("package bytes");
    let output = case.path("reqs/spine.package.json");
    assert_eq!(
        document["result"],
        json!({"kind": "packaged", "package": {
            "bytes": bytes.len(), "output": output.to_string_lossy(), "handoff": null}}),
        "{document}"
    );
    assert_eq!(document["artifacts"], json!([]), "{document}");
    assert_eq!(
        &std::fs::read(&output).expect("the file at the named path"),
        bytes
    );

    let bare = case.json("reqs/bare.json", &request("parcel.fsl"));
    let unwritten = plain(&bare);
    assert_eq!(
        unwritten.expect("package", "success", 0)["result"],
        json!({"kind": "packaged", "package": {
            "bytes": bytes.len(), "output": null, "handoff": null}})
    );

    let cli = case.path("cli.package.json");
    let overrides = RequestOverrides {
        output: Some(&cli),
        ..RequestOverrides::default()
    };
    let given = compile(&bare, None, &overrides);
    assert_eq!(
        given.expect("package", "success", 0)["result"]["package"]["output"],
        json!(cli.to_string_lossy())
    );
    assert!(cli.exists());

    let mut with_handoff = request("parcel.fsl");
    with_handoff["request"]["handoff"] = json!({"directory": "handoff", "package_name": "parcel",
        "native_join": {"work_units": 1_u64 << 40, "storage": 1_u64 << 24}});
    let handed = plain(&case.json("reqs/handoff.json", &with_handoff));
    let handoff = case.path("reqs/handoff");
    assert_eq!(
        handed.expect("package", "success", 0)["result"],
        json!({"kind": "packaged", "package": {
            "bytes": bytes.len(), "output": null, "handoff": handoff.to_string_lossy()}})
    );
    assert!(handoff.is_dir());
}

/// FR-286-AC-39: a compile refusal and a `check` outcome have `result` `null`.
#[test]
#[trace("TC-3203", "FR-286-AC-39")]
fn compile_refusal_and_check_have_a_null_result() {
    let (_case, path) = parcel(b"\xff");
    let refused = plain(&path);
    assert_eq!(
        refused.document["result"],
        Value::Null,
        "{}",
        refused.document
    );
    let checked = fsl_command::command::check_outcome(
        &_case.write("check.fsl", SOURCE.as_bytes()),
        &fsl_command::command::CheckInputs::default(),
    );
    let document: Value = serde_json::from_slice(&checked.document().to_bytes().unwrap()).unwrap();
    assert_eq!(document["result"], Value::Null, "{document}");
}

/// FR-865-AC-12 to AC-15: the text of each compile document, rendered from the
/// document alone.
#[test]
#[trace(
    "TC-3198",
    "FR-865-AC-12",
    "FR-865-AC-13",
    "FR-865-AC-14",
    "FR-865-AC-15"
)]
fn compile_text_says_what_was_produced_and_where() {
    let case = Case::new();
    case.write("reqs/parcel.fsl", SOURCE.as_bytes());
    let mut member = request("parcel.fsl");
    member["request"]["output"] = json!({"package": "spine.package.json"});
    let with_output = case.json("reqs/with-output.json", &member);
    let bytes = plain(&case.json("reqs/bare.json", &request("parcel.fsl")))
        .package
        .expect("package bytes")
        .len();
    let header = "package: success (exit 0) at stage packaging\n";
    let produced = format!("produced: checked package ({bytes} bytes)\n");

    let output = case.path("reqs/spine.package.json");
    assert_eq!(
        text_of(&with_output, &RequestOverrides::default()),
        format!(
            "{header}{produced}written to: {}\n",
            output.to_string_lossy()
        )
    );
    assert_eq!(
        text_of(&case.path("reqs/bare.json"), &RequestOverrides::default()),
        format!(
            "{header}{produced}nothing written: no --output option, output member or handoff member was given\n"
        )
    );
    let mut with_handoff = request("parcel.fsl");
    with_handoff["request"]["handoff"] = json!({"directory": "handoff", "package_name": "parcel",
        "native_join": {"work_units": 1_u64 << 40, "storage": 1_u64 << 24}});
    let handoff = case.path("reqs/handoff");
    assert_eq!(
        text_of(
            &case.json("reqs/handoff.json", &with_handoff),
            &RequestOverrides::default()
        ),
        format!(
            "{header}{produced}handoff written to: {}\n",
            handoff.to_string_lossy()
        )
    );
}

/// FR-865-AC-17: the compile request with `models` `[]` over a unit selecting
/// a model refuses `missing_import`/`missing-selection` with `selection`
/// `model`, that row's hint, and the hint in its text.
#[test]
#[trace("TC-3199", "FR-865-AC-17", "FR-865-AC-24", "FR-286-AC-40")]
fn compile_without_the_selected_model_carries_the_model_selection() {
    let unit = "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"filament.model.complete/v1\";\n\
        model Config = \"example/config-version\";\n\
        invariant NoCycle using v on Config::ConfigVersion at current { not reaches(self, self, parent) }\n";
    let (case, path) = parcel(unit.as_bytes());
    let outcome = plain(&path);
    let diagnostic = outcome.refused("package", 20, "missing_import");
    let hint = "Supply the model in the request's models member, or with --model <identity>=<file> on fsl check.";
    assert_eq!(diagnostic["cause"], "missing-selection", "{diagnostic}");
    assert_eq!(diagnostic["selection"], "model", "{diagnostic}");
    assert_eq!(diagnostic["hint"], hint, "{diagnostic}");
    drop(case);
}
