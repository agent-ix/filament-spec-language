// SPDX-License-Identifier: AGPL-3.0-or-later
//! Probability claims from original source through public compile/analyze routes.

use fsl_command::command;
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn corpus(file: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/corpus/probability")
        .join(file)
}

fn fixture(file: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tools/fsl-probability-check/tests/fixtures")
        .join(file)
}

fn source(file: &Path) -> Value {
    json!({"file":file,"authority":"test","identity":"test:probability-corpus",
        "document":"ProbabilityCorpus","formal_revision":1})
}

fn request(file: &Path) -> Value {
    json!({"format":"native-compile/1","request":{
        "program":{"source":source(file)},"models":[{"format":"semantic-ir/2.0.0",
            "source":source(&fixture("link.semantic-ir.json"))}]}})
}

fn write(directory: &Path, value: &Value) -> PathBuf {
    let file = directory.join("request.json");
    std::fs::write(&file, serde_json::to_vec(value).unwrap()).unwrap();
    file
}

#[test]
#[trace("FR-186-AC-1", "FR-185-AC-7")]
fn original_probability_forms_and_random_declarations_compile() {
    let directory = tempfile::tempdir().unwrap();
    for name in [
        "bounded-random",
        "bounded-every-scheduler",
        "reachability",
        "quantile",
        "mean",
        "long-run",
    ] {
        let file = corpus(&format!("original/{name}.native"));
        let output = command::compile_outcome(&write(directory.path(), &request(&file)))
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(!output.package_bytes().is_empty(), "{name}");
    }
}

#[test]
#[trace(
    "FR-186-AC-2",
    "FR-186-AC-3",
    "FR-186-AC-4",
    "FR-186-AC-5",
    "FR-186-AC-9"
)]
fn invalid_probability_claim_forms_refuse_with_their_specific_causes() {
    let directory = tempfile::tempdir().unwrap();
    let original = std::fs::read_to_string(corpus("original/bounded-random.native")).unwrap();
    for (name, from, to, code, cause) in [
        (
            "qualitative-zero",
            "probability >= 0.8",
            "probability >= 0",
            "invalid_probabilistic_model",
            "qualitative-threshold",
        ),
        (
            "qualitative-one",
            "probability >= 0.8",
            "probability >= 1",
            "invalid_probabilistic_model",
            "qualitative-threshold",
        ),
        (
            "percent",
            "probability >= 0.8",
            "probability >= 80",
            "invalid_probabilistic_model",
            "qualitative-threshold",
        ),
        (
            "unbounded-event",
            "eventually[0,2]",
            "eventually",
            "unsupported_construct",
            "expression-form",
        ),
        (
            "non-object-over",
            "packet: Network::Msg",
            "packet: Boolean",
            "invalid_probabilistic_model",
            "over-not-object-type",
        ),
        (
            "partial-confidence",
            "on origin {",
            "on origin with alpha 0.1 {",
            "invalid_probabilistic_model",
            "partial-confidence",
        ),
        (
            "invalid-confidence",
            "on origin {",
            "on origin with alpha 0.1, beta 0.1, indifference 0.3 {",
            "invalid_probabilistic_model",
            "invalid-confidence",
        ),
    ] {
        assert!(original.contains(from));
        let file = directory.path().join(format!("{name}.native"));
        std::fs::write(&file, original.replacen(from, to, 1)).unwrap();
        let error = match command::compile_outcome(&write(directory.path(), &request(&file))) {
            Ok(_) => panic!("{name}: expected {code}/{cause}"),
            Err(error) => error,
        };
        let artifact = error.artifact_value().unwrap();
        assert_eq!(artifact["code"], code, "{name}: {artifact}");
        assert_eq!(artifact["cause"], cause, "{name}: {artifact}");
    }
}
