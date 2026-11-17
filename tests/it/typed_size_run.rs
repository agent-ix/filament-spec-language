// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-981-AC-4 (TC-2775): `size<N>(c)` runs to the same length as
//! `size(c)`, through `command::run` over FR-108's ConfigVersion spine.
use crate::command_harness::run_file;
use crate::support::config_version_spine::{spine, Case};
use ix_trace_rs::trace;
use serde_json::{json, Value};

fn source(file: &str, document: &str) -> Value {
    json!({
        "file": file,
        "authority": "agent-ix", "identity": format!("test:fr-981:{file}"),
        "document": document, "formal_revision": 1
    })
}

#[trace("TC-2775", "FR-981-AC-4", "FSpec-TC-1900", "FSpec-FR-783-AC-3")]
#[test]
#[ignore = "QSL-1966: S2 refuses size<T>(..); running it is not implemented yet"]
fn a_typed_size_runs_to_the_untyped_length() {
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::Healthy).unwrap();
    let unit = std::fs::read_to_string(directory.path().join("spine-unit.native")).unwrap();
    std::fs::write(
        directory.path().join("spine-unit.native"),
        format!(
            "{unit}function three using v(): Boolean pure \
             {{ size<Int[0, 4]>(sequence[7, 8, 7]) = 3 and size(sequence[7, 8, 7]) = 3 }}\n"
        ),
    )
    .unwrap();
    let request = json!({"format": "native-run/1", "request": {
        "models": [{"format": "semantic-ir/2.0.0", "source":
            source("spine-model.semantic-ir.json", "SpineModel")}],
        "program": {"source": source("spine-unit.native", "SpineUnit")},
        "clause": {
            "selection": {"function": {"name": "three", "arguments": [], "snapshot": "current"}},
            "observations": {"current": "spine-snapshot.json"},
        },
    }});
    let path = directory.path().join("request.json");
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    let output = run_file(&path);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
}
