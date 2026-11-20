// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2474: `fsl export <request>` at the command surface the driver calls
//! (`run_with_overrides` with `--output` and `--limit` as request overrides):
//! an assembled export, a below-floor refusal, an exhausted output budget,
//! output protection, limit refusals, and odd input.

use fsl_command::command::RequestOverrides;
use fsl_replay::spine::SpineLimits;
use ix_trace_rs::trace;
use serde_json::{json, Value};

use super::{export, Case, INVALID_UTF8};

const BRAKING: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile p = \"filament.temporal.infinite-trace/v1\";\n\
     hybrid Braking using p {\n\
       mode Moving invariant { v >= 0 and v <= 20 and a >= 1 and a <= 2 }\n\
         flow { v' = -a; a' = 0; }\n\
         transition to Stopped when { v = 0 } reset { v = 0 };\n\
       mode Stopped invariant { v = 0 }\n\
         flow { v' = 0; a' = 0; }\n\
     }\n\
     temporal SlowEnough using p over (h: Braking) clock \"model-time\" \
     on origin { always holds(h.v < 30) }\n";

fn member() -> Value {
    json!({"profile": "filament.output.tla2/v1", "obligations": ["SlowEnough"],
        "minimum_fidelity": "any"})
}

fn braking(source: &[u8], member: Value) -> (Case, std::path::PathBuf) {
    let case = Case::new();
    case.write("unit.native", source);
    let request = json!({"format": "native-run/1", "request": {"models": [],
        "program": {"source": {"file": "unit.native", "authority": "export-contracts",
            "identity": "contracts:braking", "document": "Braking", "formal_revision": 1}},
        "export": member}});
    let path = case.json("export.json", &request);
    (case, path)
}

fn plain(path: &std::path::Path) -> super::Outcome {
    export(path, &RequestOverrides::default())
}

/// FR-286-AC-22, FR-027: an assembled export writes one `export` outcome,
/// success at S4, `result.kind` `exported`, exit 0.
#[test]
#[trace("FR-286-AC-22", "FR-027")]
fn an_assembled_export_exits_0() {
    let (_case, path) = braking(BRAKING.as_bytes(), member());
    let outcome = plain(&path);
    let document = outcome.expect("export", "success", 0);
    assert_eq!(document["last_stage"], "S4", "{document}");
    assert_eq!(document["result"]["kind"], "exported", "{document}");
}

/// `--output <file>`: the override writes the target, and a second run over
/// the existing path refuses `invalid-request`, exit 20, `last_stage` null,
/// leaving the bytes unchanged.
#[test]
#[trace("FR-286-AC-22", "FR-810")]
fn output_override_writes_once_and_never_overwrites() {
    let (case, path) = braking(BRAKING.as_bytes(), member());
    let target = case.path("braking.tla");
    let overrides = RequestOverrides {
        output: Some(&target),
        ..RequestOverrides::default()
    };
    export(&path, &overrides).expect("export", "success", 0);
    let bytes = std::fs::read(&target).expect("the target file");
    assert!(!bytes.is_empty());

    let again = export(&path, &overrides);
    again.refused("export", 20, "invalid-request");
    assert!(again.document["last_stage"].is_null(), "{}", again.document);
    assert_eq!(std::fs::read(&target).unwrap(), bytes);
}

/// `--output` beside an `export.output` member refuses, exit 20, and writes
/// neither file.
#[test]
#[trace("FR-810")]
fn output_override_beside_an_output_member_refuses() {
    let mut with_output = member();
    with_output["output"] = json!({"file": "member.tla"});
    let (case, path) = braking(BRAKING.as_bytes(), with_output);
    let target = case.path("flag.tla");
    let overrides = RequestOverrides {
        output: Some(&target),
        ..RequestOverrides::default()
    };
    let outcome = export(&path, &overrides);
    assert_eq!(outcome.exit, 20, "{}", outcome.document);
    assert!(!target.exists());
    assert!(!case.path("member.tla").exists());
}

/// FR-1015-AC-1: below the default fidelity floor the export is
/// unsupported, exit 21, and writes no file.
#[test]
#[trace("FR-1015-AC-1")]
fn an_export_below_its_floor_is_unsupported() {
    let mut floor = member();
    floor.as_object_mut().unwrap().remove("minimum_fidelity");
    floor["output"] = json!({"file": "m.tla"});
    let (case, path) = braking(BRAKING.as_bytes(), floor);
    plain(&path).expect("export", "unsupported", 21);
    assert!(!case.path("m.tla").exists());
}

/// FR-1015-AC-2 (`--limit export.output_bytes=1`): the override stops the
/// export `incomplete`, exit 22, with no file; the same setting in the
/// member and the override refuses, exit 20.
#[test]
#[trace("FR-1015-AC-2", "FR-255-AC-22")]
fn the_output_byte_limit_is_reached_through_the_override() {
    let (case, path) = braking(BRAKING.as_bytes(), member());
    let target = case.path("limited.tla");
    let mut limits = SpineLimits::default();
    limits.export.output_bytes = Some(1);
    let overrides = RequestOverrides {
        output: Some(&target),
        limits: Some(&limits),
        ..RequestOverrides::default()
    };
    export(&path, &overrides).expect("export", "incomplete", 22);
    assert!(!target.exists());

    let mut limited = member();
    limited["limits"] = json!({"export.output_bytes": 1});
    let (_case, path) = braking(BRAKING.as_bytes(), limited);
    let overrides = RequestOverrides {
        limits: Some(&limits),
        ..RequestOverrides::default()
    };
    let outcome = export(&path, &overrides);
    assert_eq!(outcome.exit, 20, "{}", outcome.document);
}

/// FR-255: a `limits` entry that is not a setting name refuses
/// `invalid-request`, exit 20.
#[test]
#[trace("FR-255", "FR-1015-AC-2")]
fn an_unknown_limit_name_refuses() {
    let mut unknown = member();
    unknown["limits"] = json!({"output_bytes": 1});
    let (_case, path) = braking(BRAKING.as_bytes(), unknown);
    let outcome = plain(&path);
    assert_eq!(outcome.exit, 20, "{}", outcome.document);
    assert!(
        outcome.document.to_string().contains("invalid-request"),
        "{}",
        outcome.document
    );
}

/// FR-001: invalid UTF-8 in the program, and a missing, a directory and an
/// empty request operand, each refuse, exit 20, with one diagnostic.
#[test]
#[trace("FR-001-AC-3", "FR-001")]
fn odd_input_refuses() {
    let (case, path) = braking(INVALID_UTF8, member());
    plain(&path).expect("export", "refusal", 20);
    std::fs::create_dir_all(case.path("folder")).unwrap();
    for path in [
        case.path("absent.json"),
        case.path("folder"),
        case.write("empty.json", b""),
    ] {
        let outcome = plain(&path);
        outcome.expect("export", "refusal", 20);
        assert_eq!(
            outcome.document["diagnostics"].as_array().map(Vec::len),
            Some(1),
            "{}",
            outcome.document
        );
    }
}
