// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-2062 (FR-185-AC-12): a package-stage refusal that concerns a random
//! parameter, a workload, a reward or a delay carries the declaration's
//! source reference and label span in its locus member and names the
//! declaration in words, through the public compile command.

use std::path::Path;

use fsl_command::command;
use fsl_replay::Operation;
use ix_trace_rs::trace;
use serde_json::{json, Value};

const DOMAIN: &[u8] = include_bytes!("../../../examples/config-version/model.semantic-ir.json");

/// The ConfigVersion unit with a workload and a reward on `attemptUpdate`.
const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"filament.model.complete/v1\";\n\
     model Config = \"example/config-version\";\n\
     workload Steady on Config { weight ConfigVersion::attemptUpdate = 1; }\n\
     reward cost on ConfigVersion::attemptUpdate = 1;\n";

fn compile_request(directory: &Path) -> std::path::PathBuf {
    std::fs::write(directory.join("program.native"), UNIT).unwrap();
    std::fs::write(directory.join("model.json"), DOMAIN).unwrap();
    let path = directory.join("compile.json");
    let request = json!({"format": "native-compile/1", "request": {
        "models": [{"format": "semantic-ir/2.0.0", "source": {
            "file": "model.json", "authority": "model issuer", "identity": "model:source",
            "document": "DomainModel", "formal_revision": 1}}],
        "program": {"source": {"file": "program.native", "authority": "offered issuer",
            "identity": "offered:unit", "document": "ProbabilisticUnit", "formal_revision": 1}}}});
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    path
}

/// The package stage either emits the unit (FR-740) or refuses it with the
/// workload's locus; it never refuses with a null locus or a message that
/// carries a debug rendering of a span or source reference.
#[trace("TC-2062", "FR-185-AC-12")]
#[test]
#[ignore = "QSL-1996: package-stage refusals of probabilistic declarations carry the declaration locus not implemented yet"]
fn a_package_stage_refusal_of_a_workload_carries_its_locus() {
    let directory = tempfile::tempdir().unwrap();
    let path = compile_request(directory.path());
    let Err(error) = command::compile_outcome(&path) else {
        return;
    };
    let outcome: Value = serde_json::from_slice(
        &error
            .outcome_document(Operation::Package)
            .to_bytes()
            .unwrap(),
    )
    .unwrap();
    let diagnostic = &outcome["diagnostics"][0];
    assert!(
        diagnostic["locus"].is_object(),
        "a non-null locus: {outcome}"
    );
    let message = diagnostic["message"].as_str().unwrap_or_default();
    assert!(!message.contains("Span {"), "{message}");
    assert!(!message.contains("RawSourceRef"), "{message}");
    assert!(message.contains("workload"), "{message}");
}
