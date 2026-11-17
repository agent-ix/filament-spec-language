// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-981-AC-4 (TC-2775): `size<N>(c)` runs to the same length as
//! `size(c)`, through `command::run` over FR-108's ConfigVersion spine. The
//! sequence literal takes its type from an annotated `let` (FSpec FR-144
//! "Collection literal typing"); as a bare `size` argument it has none.
use crate::command_harness::{run_file, Output};
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

/// Runs the function claim `three` with `body` over the healthy spine.
fn run_three(body: &str) -> Output {
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::Healthy).unwrap();
    let unit = std::fs::read_to_string(directory.path().join("spine-unit.native")).unwrap();
    std::fs::write(
        directory.path().join("spine-unit.native"),
        format!("{unit}function three using v(): Boolean pure {{ {body} }}\n"),
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
    run_file(&path)
}

#[trace("TC-2775", "FR-981-AC-4", "FSpec-TC-1900", "FSpec-FR-783-AC-3")]
#[test]
#[ignore = "QSL-1966: S2 refuses size<T>(..); running it is not implemented yet"]
fn a_typed_size_runs_to_the_untyped_length() {
    let output = run_three(
        "let q: Sequence<Integer>[0, 5] = sequence[7, 8, 7] in \
         size<Int[0, 5]>(q) = 3 and size(q) = 3",
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
}

#[trace("TC-2775", "FR-981-AC-4", "FSpec-TC-189", "FSpec-FR-144-AC-10")]
#[test]
#[ignore = "QSL-1966: S2 refuses size<T>(..) before the literal is typed; not implemented yet"]
fn a_bare_literal_size_argument_is_an_ambiguous_literal() {
    let output = run_three("size<Int[0, 4]>(sequence[7, 8, 7]) = 3");
    assert_ne!(output.status.code(), Some(0), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("ill_typed"), "{stdout}");
    assert!(stdout.contains("ambiguous-literal"), "{stdout}");
}
