// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-665-AC-5: inspect refusals name the caller's raw handoff operand and
//! the member file and pointer that caused the refusal.

use std::path::{Path, PathBuf};

use fsl_command::checked_handoff::{self, CheckedHandoffRequest, Corpus};
use fsl_command::command;
use fsl_command::emit_checked_v2::Request;
use ix_trace_rs::trace;
use serde_json::{json, Value};

fn source() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/spine-compile.native")
}

fn write_handoff(directory: &Path) {
    let source_file =
        tempfile::NamedTempFile::new_in(env!("CARGO_MANIFEST_DIR")).expect("temporary source file");
    std::fs::copy(source(), source_file.path()).expect("copy source fixture");
    let source_path = PathBuf::from(
        source_file
            .path()
            .file_name()
            .expect("temporary source has a file name"),
    );
    checked_handoff::write(
        &CheckedHandoffRequest {
            emit: Request {
                source: source_path,
                package_name: "spine".to_owned(),
                models: Vec::new(),
                libraries: Vec::new(),
                execution_points: None,
                temporal_selections: Vec::new(),
                out: None,
            },
            corpus: Corpus::None,
            native_join: fsl_package::NativeJoinLimits::new(1_000_000, 1_000_000),
        },
        directory,
    )
    .expect("valid checked handoff is written");
}

fn refusal(directory: &Path, operand: &str) -> Value {
    let document = command::inspect_handoff_with_operand(directory, operand);
    let value: Value =
        serde_json::from_slice(&document.to_bytes().expect("outcome bytes")).expect("outcome JSON");
    assert_eq!(value["operation"], "inspect", "{value}");
    assert_eq!(value["category"], "refusal", "{value}");
    assert_eq!(value["diagnostics"].as_array().unwrap().len(), 1, "{value}");
    value
}

fn assert_input_locus(value: &Value, operand: &str, member: Option<&str>, pointer: Option<&str>) {
    assert_eq!(
        value["diagnostics"][0]["locus"],
        json!({
            "kind": "input",
            "member": member,
            "operand": operand,
            "pointer": pointer,
        }),
        "{value}"
    );
}

/// The exact caller operand survives inspection, and each listed refusal
/// names its input member and reader pointer as specified by FR-665-AC-5.
#[trace("TC-1748", "FR-665-AC-5")]
#[test]
fn handoff_refusal_loci_name_operand_member_and_pointer() {
    let root = tempfile::tempdir().expect("temporary root");
    let handoff = root.path().join("handoff");
    write_handoff(&handoff);

    let selection = handoff.join("selection.json");
    let selection_bytes = std::fs::read(&selection).expect("selection bytes");
    std::fs::remove_file(&selection).expect("remove selection");
    let missing = refusal(&handoff, "./handoff/");
    assert_eq!(missing["diagnostics"][0]["code"], "handoff-read");
    assert_input_locus(&missing, "./handoff/", Some("selection.json"), None);
    std::fs::write(&selection, &selection_bytes).expect("restore selection");

    let absent = root.path().join("absent");
    let missing_directory = refusal(&absent, "absent");
    assert_eq!(missing_directory["diagnostics"][0]["code"], "handoff-read");
    assert_input_locus(&missing_directory, "absent", None, None);

    let mut changed_selection: Value = serde_json::from_slice(&selection_bytes).unwrap();
    let counts = changed_selection["package"]["node_counts"]
        .as_object_mut()
        .expect("selection node counts");
    let (_, count) = counts.iter_mut().next().expect("at least one node count");
    *count = json!(count.as_u64().unwrap() + 1);
    std::fs::write(&selection, serde_json::to_vec(&changed_selection).unwrap())
        .expect("write changed selection");
    let mismatch = refusal(&handoff, "handoff");
    assert_eq!(
        mismatch["diagnostics"][0]["code"],
        "handoff-read-back-differs"
    );
    assert_input_locus(&mismatch, "handoff", Some("selection.json"), None);

    std::fs::write(&selection, &selection_bytes).expect("restore selection");
    let package = handoff.join("checked-package.json");
    let mut changed_package: Value =
        serde_json::from_slice(&std::fs::read(&package).expect("package bytes")).unwrap();
    changed_package["contract_version"] = json!("filament.checked-package/v1");
    std::fs::write(&package, serde_json::to_vec(&changed_package).unwrap())
        .expect("write wrong package version");
    let reader = refusal(&handoff, "handoff");
    assert_eq!(reader["diagnostics"][0]["code"], "handoff-reader");
    assert_input_locus(
        &reader,
        "handoff",
        Some("checked-package.json"),
        Some("/contract_version"),
    );
}
