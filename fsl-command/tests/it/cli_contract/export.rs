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

/// The Counter unit: one model invariant the export obligation names.
pub(super) const COUNTER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"filament.value.complete/v1\";\n\
     profile m = \"filament.model.complete/v1\";\n\
     model Queue = \"test/counter\";\n\
     pre SpaceAvailable using v on Queue::Counter::inc { self.value < 2 }\n\
     post AppendOne using v on Queue::Counter::inc \
     { pre(self.value) < 2 and self.value = pre(self.value) + 1 }\n\
     invariant Bounded using m on Queue::Counter at current { self.value <= 2 }\n";

const MODEL: &[u8] = include_bytes!("../../fixtures/cli-contract/counter.semantic-ir.json");

pub(super) fn member() -> Value {
    json!({"profile": "filament.output.tla2/v1", "obligations": ["Bounded"],
        "minimum_fidelity": "any", "initial": "start.json"})
}

/// Write the model, the initial snapshot and the unit `source` into `case`
/// and return the `native-run/1` request that exports under `member`.
pub(super) fn counter_request(case: &Case, source: &[u8], member: Value) -> Value {
    case.write("unit.native", source);
    case.write("counter.json", MODEL);
    case.json(
        "start.json",
        &json!({"format": "filament.state.snapshot/v1",
            "identity": {"authority": "contracts", "identity": "queue-start"},
            "observation": "current", "model": {"identity": "test/counter"},
            "anchor": {"kind": "initialization", "name": "queue-start"},
            "populations": [{"population": "ix://test/counter/counters", "complete": true,
                "objects": [{"key": "queue-east", "type": "ix://test/counter/Counter",
                    "fields": {"value": {"integer": "0"}}}]}]}),
    );
    json!({"format": "native-run/1", "request": {
        "models": [{"format": "semantic-ir/2.0.0", "source": {"file": "counter.json",
            "authority": "contracts", "identity": "counter-reference",
            "document": "CounterInput", "formal_revision": 1}}],
        "program": {"source": {"file": "unit.native", "authority": "export-contracts",
            "identity": "contracts:counter", "document": "Counter", "formal_revision": 1}},
        "export": member}})
}

fn braking(source: &[u8], member: Value) -> (Case, std::path::PathBuf) {
    let case = Case::new();
    let request = counter_request(&case, source, member);
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
    let (_case, path) = braking(COUNTER.as_bytes(), member());
    let outcome = plain(&path);
    let document = outcome.expect("export", "success", 0);
    assert_eq!(document["last_stage"], "S4", "{document}");
    assert_eq!(document["result"]["kind"], "exported", "{document}");
}

/// A genuine assembled package reaches evidence admission, which reports
/// the build's pending capability without claiming package or proof verification.
#[test]
#[trace("FR-1022", "TC-2872")]
fn an_assembled_package_reaches_pending_evidence_admission() {
    use fsl_command::command::{export_with_limits_and_overrides, ExportSelection, RunLimits};

    let (case, path) = braking(COUNTER.as_bytes(), member());
    let ordinary = plain(&path);
    let exported = ordinary.expect("export", "success", 0);
    let package = &exported["result"]["export"]["package"];
    assert!(package.is_object(), "{exported}");
    let mut input: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    input["request"].as_object_mut().unwrap().remove("export");
    input["request"]["export_evidence"] = json!({
        "package": package, "obligation": "Bounded", "tool": "lean",
        "artifact": "not-read-until-settlement-is-implemented.lean"
    });
    let path = case.json("evidence.json", &input);
    let result = export_with_limits_and_overrides(
        &path,
        ExportSelection::Evidence,
        RunLimits::default(),
        &RequestOverrides::default(),
    )
    .unwrap();
    assert_eq!(result.exit_code, 21);
    let document =
        serde_json::to_value(result.outcome.expect("evidence lifecycle outcome")).unwrap();
    super::assert_outcome_schema(&document);
    assert_eq!(document["category"], "unsupported", "{document}");
    assert!(document["result"].is_null(), "{document}");
    assert!(document["last_stage"].is_null(), "{document}");
    assert_eq!(
        document["diagnostics"][0]["code"],
        "unimplemented_capability"
    );
    assert_eq!(document["diagnostics"][0]["cause"], "unimplemented-meaning");
    assert!(
        !document.to_string().contains("tool-unavailable"),
        "{document}"
    );

    let generic = fsl_command::command::run(&path, &crate::negotiation::Fr290).unwrap();
    assert_eq!(generic.exit_code, 21);
    assert_eq!(
        serde_json::to_value(generic.outcome.unwrap()).unwrap(),
        document
    );
}

/// Explicit selection retains FSL's dependent bounds, output ownership and stage limits.
#[test]
#[trace("FR-1015", "FR-026", "FR-810", "TC-104")]
fn explicit_export_selection_preserves_bounds_and_output_overrides() {
    use fsl_command::command::{export_with_limits_and_overrides, ExportSelection, RunLimits};

    let (case, path) = braking(COUNTER.as_bytes(), member());
    let target = case.path("selected.tla");
    let overrides = RequestOverrides {
        output: Some(&target),
        ..RequestOverrides::default()
    };
    let limited = export_with_limits_and_overrides(
        &path,
        ExportSelection::Ordinary,
        RunLimits {
            dependent_bytes: 1,
            ..RunLimits::default()
        },
        &overrides,
    )
    .unwrap_err();
    assert_eq!(limited.category().exit_code(), 22);
    assert!(!target.exists());

    let mut spine = SpineLimits::default();
    spine.export.output_bytes = Some(1);
    let exhausted = export_with_limits_and_overrides(
        &path,
        ExportSelection::Ordinary,
        RunLimits::default(),
        &RequestOverrides {
            limits: Some(&spine),
            ..overrides
        },
    )
    .unwrap();
    assert_eq!(exhausted.exit_code, 22);
    assert!(!target.exists());

    let result = export_with_limits_and_overrides(
        &path,
        ExportSelection::Ordinary,
        RunLimits::default(),
        &overrides,
    )
    .unwrap();
    assert_eq!(result.exit_code, 0);
    let document = serde_json::to_value(result.outcome.unwrap()).unwrap();
    assert_eq!(document["result"]["kind"], "exported");
    let text = document["result"]["export"]["target_text"]
        .as_str()
        .unwrap();
    assert_eq!(std::fs::read(target).unwrap(), text.as_bytes());
}

/// `--output <file>`: the override writes the target, and a second run over
/// the existing path refuses `invalid-request`, exit 20, `last_stage` null,
/// leaving the bytes unchanged.
#[test]
#[trace("FR-286-AC-22", "FR-810")]
fn output_override_writes_once_and_never_overwrites() {
    let (case, path) = braking(COUNTER.as_bytes(), member());
    let target = case.path("counter.tla");
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
    let (case, path) = braking(COUNTER.as_bytes(), with_output);
    let target = case.path("flag.tla");
    let overrides = RequestOverrides {
        output: Some(&target),
        ..RequestOverrides::default()
    };
    let outcome = export(&path, &overrides);
    let refusal = outcome.refused("export", 20, "invalid-request");
    let text = refusal.to_string();
    assert!(
        text.contains("output") && text.contains("--output"),
        "{refusal}"
    );
    assert!(!target.exists());
    assert!(!case.path("member.tla").exists());
}

/// FR-1015-AC-6: a refused record fails the default fidelity floor, the export is
/// unsupported, exit 21, and writes no file.
#[test]
#[trace("FR-1015-AC-6")]
fn an_export_below_its_floor_is_unsupported() {
    let mut floor = member();
    floor.as_object_mut().unwrap().remove("minimum_fidelity");
    floor["output"] = json!({"file": "m.tla"});
    floor["obligations"] = json!(["Nope", "Bounded"]);
    let (case, path) = braking(COUNTER.as_bytes(), floor);
    plain(&path).expect("export", "unsupported", 21);
    assert!(!case.path("m.tla").exists());
}

/// FR-1015-AC-2 (`--limit export.output_bytes=1`): the override stops the
/// export `incomplete`, exit 22, with no file; the same setting in the
/// member and the override refuses, exit 20.
#[test]
#[trace("FR-1015-AC-2", "FR-255-AC-22")]
fn the_output_byte_limit_is_reached_through_the_override() {
    let (case, path) = braking(COUNTER.as_bytes(), member());
    let target = case.path("limited.tla");
    let mut limits = SpineLimits::default();
    limits.export.output_bytes = Some(1);
    let overrides = RequestOverrides {
        output: Some(&target),
        limits: Some(&limits),
        ..RequestOverrides::default()
    };
    let limited_run = export(&path, &overrides);
    limited_run.expect("export", "incomplete", 22);
    assert!(
        limited_run
            .document
            .to_string()
            .contains("export.output_bytes"),
        "{}",
        limited_run.document
    );
    assert!(!target.exists());

    let mut limited = member();
    limited["limits"] = json!({"export.output_bytes": 1});
    let (_case, path) = braking(COUNTER.as_bytes(), limited);
    let overrides = RequestOverrides {
        limits: Some(&limits),
        ..RequestOverrides::default()
    };
    let outcome = export(&path, &overrides);
    let refusal = outcome.refused("export", 20, "invalid-request");
    let text = refusal.to_string();
    assert!(
        text.contains("limits") && text.contains("export.output_bytes"),
        "{refusal}"
    );
}

/// FR-255: a `limits` entry that is not a setting name refuses
/// `invalid-request`, exit 20.
#[test]
#[trace("FR-255", "FR-1015-AC-2")]
fn an_unknown_limit_name_refuses() {
    let mut unknown = member();
    unknown["limits"] = json!({"output_bytes": 1});
    let (_case, path) = braking(COUNTER.as_bytes(), unknown);
    let outcome = plain(&path);
    let refusal = outcome.refused("export", 20, "invalid-request");
    assert!(refusal.to_string().contains("output_bytes"), "{refusal}");
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
