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

/// A unit with source-declared objects and a reward on one of them. S3
/// accepts it; S4 refuses it until QSL-2463 emits source-declared objects,
/// which is the refusal whose locus and message this test reads.
const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile P = \"filament.model.complete/v1\";\n\
     object O { n: Integer; operation step(x: Integer) modifies self.n; }\n\
     reward cost on O::step = x;\n";

fn compile_request(directory: &Path) -> std::path::PathBuf {
    std::fs::write(directory.join("program.native"), UNIT).unwrap();
    let path = directory.join("compile.json");
    let request = json!({"format": "native-compile/1", "request": {
        "models": [],
        "program": {"source": {"file": "program.native", "authority": "offered issuer",
            "identity": "offered:unit", "document": "ProbabilisticUnit", "formal_revision": 1}}}});
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    path
}

/// The package stage refuses the reward with the reward's locus and a
/// message that names it in words, never with a null locus or a debug
/// rendering of a span or source reference.
#[trace("TC-2062", "FR-185-AC-12")]
#[test]
fn a_package_stage_refusal_of_a_reward_carries_its_locus() {
    let directory = tempfile::tempdir().unwrap();
    let path = compile_request(directory.path());
    let error = command::compile_outcome(&path)
        .err()
        .expect("the package stage refuses a reward on a source object");
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
    assert!(message.contains("reward"), "{message}");
}
