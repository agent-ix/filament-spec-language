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

/// A unit with a timed, source-declared object and a delay on one of its
/// operations: S3 accepts it and S4 emits the delay node (QSL-2463).
const DELAY_UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile T = \"filament.temporal.complete/v1\";\n\
     dimension Time; unit ms : Time = rational(1, 1);\ntime O dense unit ms;\n\
     object O { n: Integer; operation step(x: Integer) modifies self.n; }\n\
     delay O::step ~ uniform;\n";

/// A unit with a source-declared object and a random parameter over a
/// `String`, which has no support literal kind. S3 accepts it; S4 still
/// refuses it, which is the refusal whose locus and message this test reads.
const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"filament.model.complete/v1\";\n\
     object O { n: Integer; operation step(s: String) modifies self.n; }\n\
     random O::step(s) ~ { \"a\": 1, \"b\": 1 };\n";

fn compile_request(directory: &Path, source: &str) -> std::path::PathBuf {
    std::fs::write(directory.join("program.native"), source).unwrap();
    let path = directory.join("compile.json");
    let request = json!({"format": "native-compile/1", "request": {
        "models": [],
        "program": {"source": {"file": "program.native", "authority": "offered issuer",
            "identity": "offered:unit", "document": "ProbabilisticUnit", "formal_revision": 1}}}});
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    path
}

/// The package stage refuses the random parameter with the parameter's locus
/// and a message that names it in words, never with a null locus or a debug
/// rendering of a span or source reference.
#[trace("TC-2062", "FR-185-AC-12")]
#[test]
fn a_package_stage_refusal_of_a_random_parameter_carries_its_locus() {
    let directory = tempfile::tempdir().unwrap();
    let path = compile_request(directory.path(), UNIT);
    let error = command::compile_outcome(&path)
        .err()
        .expect("the package stage refuses a random parameter over a String");
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
    assert!(message.contains("random"), "{message}");
    assert!(message.contains("step"), "names the declaration: {message}");
    let locus = &diagnostic["locus"];
    assert_eq!(
        locus["source"],
        json!({"authority": "offered issuer", "identity": "offered:unit"}),
        "the source reference: {locus}"
    );
    let (start, end) = (
        locus["start"].as_u64().unwrap() as usize,
        locus["end"].as_u64().unwrap() as usize,
    );
    assert!(
        UNIT[start..end].contains("step"),
        "the span is the declaration's: {locus}"
    );
}

/// A delay on a timed source object is no longer a package-stage refusal: S4
/// emits it (QSL-2463), so the package stage compiles the unit.
#[trace("TC-2062", "FR-185-AC-12")]
#[test]
fn a_delay_on_a_timed_source_object_compiles_at_the_package_stage() {
    let directory = tempfile::tempdir().unwrap();
    let path = compile_request(directory.path(), DELAY_UNIT);
    if let Err(error) = command::compile_outcome(&path) {
        let outcome = error
            .outcome_document(Operation::Package)
            .to_bytes()
            .unwrap();
        panic!("the delay compiles: {}", String::from_utf8_lossy(&outcome));
    }
}
